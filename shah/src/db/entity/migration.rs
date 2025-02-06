use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
    marker::PhantomData,
    os::unix::fs::FileExt,
    path::Path,
};

use super::{
    DbHead, EntityItem, EntityMeta, Gene, GeneId, Schema, ENTITY_MAGIC,
};
use crate::{db::entity::META_OFFSET, utils, DbError, ShahError};
use crate::{models::Binary, NotFound};

pub trait EntityMigrateFrom<Old: EntityItem, State = ()>: Sized {
    fn entity_migrate_from(old: Old, state: State) -> Result<Self, ShahError>;
}

impl<T: EntityItem, S> EntityMigrateFrom<T, S> for T {
    fn entity_migrate_from(old: T, _: S) -> Result<Self, ShahError> {
        Ok(old)
    }
}

#[derive(Debug)]
pub struct EntityMigration<Old: EntityItem, State> {
    pub from: EntityMigrationDb<Old>,
    pub state: State,
}

#[derive(Debug)]
pub struct EntityMigrationDb<T: EntityItem> {
    file: File,
    iteration: u16,
    name: String,
    total: u64,
    _e: PhantomData<T>,
    ls: String,
}

impl<T: EntityItem> EntityMigrationDb<T> {
    pub fn new(path: &str, iteration: u16) -> Result<Self, ShahError> {
        let path = Path::new("data/").join(path);
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path: {path}");

        utils::validate_db_name(name)?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .truncate(false)
            .open(path.join(format!("{name}.{iteration}.shah")))?;

        let mut db = Self {
            file,
            iteration,
            name: name.to_string(),
            total: 0,
            _e: PhantomData::<T>,
            ls: format!("<EntityMigrationDb {name}.{iteration}>"),
        };

        db.init()?;

        Ok(db)
    }

    pub fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    fn init(&mut self) -> Result<(), ShahError> {
        let file_size = self.file_size()?;
        if file_size < META_OFFSET + T::N {
            log::error!("{} content is not valid", self.ls);
            return Err(DbError::InvalidDbContent)?;
        }

        self.check_db_head()?;
        self.check_schema()?;

        Ok(())
    }

    fn check_db_head(&mut self) -> Result<(), ShahError> {
        let mut head = DbHead::default();

        self.file.read_exact_at(head.as_binary_mut(), 0)?;
        if head.magic != ENTITY_MAGIC {
            log::error!(
                "{} invalid db magic: {:?} != {ENTITY_MAGIC:?}",
                self.ls,
                head.magic
            );
            return Err(DbError::InvalidDbHead)?;
        }
        if head.iteration != self.iteration {
            log::error!(
                "{} invalid {} != {}",
                self.ls,
                head.iteration,
                self.iteration
            );
            return Err(DbError::InvalidDbHead)?;
        }

        Ok(())
    }

    fn check_schema(&mut self) -> Result<(), ShahError> {
        let mut schema = EntityMeta::default();

        self.file.read_exact_at(schema.as_binary_mut(), DbHead::N)?;
        if schema.item_size != T::N {
            log::error!(
                "{} schema.item_size != current item size. {} != {}",
                self.ls,
                schema.item_size,
                T::N
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        let schema = Schema::decode(&schema.schema)?;
        if schema != T::shah_schema() {
            log::error!(
                "{} mismatch schema.
                    did you forgot to update the iternation?",
                self.ls,
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        Ok(())
    }

    pub fn seek_id(&mut self, id: GeneId) -> Result<(), ShahError> {
        let pos = META_OFFSET + id * T::N;
        self.file.seek(SeekFrom::Start(pos))?;
        Ok(())
    }

    pub fn read(&mut self, entity: &mut T) -> Result<(), ShahError> {
        match self.file.read_exact(entity.as_binary_mut()) {
            Ok(_) => {}
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => Err(NotFound::OutOfBounds)?,
                _ => Err(e)?,
            },
        }

        Ok(())
    }

    pub fn get(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        gene.validate()?;

        self.seek_id(gene.id)?;
        self.read(entity)?;

        if !entity.is_alive() {
            return Err(NotFound::EntityNotAlive)?;
        }

        gene.check(entity.gene())?;

        Ok(())
    }
}
