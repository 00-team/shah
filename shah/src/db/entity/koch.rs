use std::cell::{RefCell, RefMut};
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::os::unix::fs::FileExt;
use std::path::Path;

use super::{EntityHead, EntityItem, ENTITY_MAGIC, META_OFFSET};
use crate::models::{Binary, Gene, GeneId, Schema};
use crate::{utils, DbError, NotFound, ShahError};

pub trait EntityKochFrom<Old: EntityItem, State = ()>: Sized {
    fn entity_koch_from(
        old: Old, state: RefMut<State>,
    ) -> Result<Self, ShahError>;
}

impl<T: EntityItem, S> EntityKochFrom<T, S> for T {
    fn entity_koch_from(old: T, _: RefMut<S>) -> Result<Self, ShahError> {
        Ok(old)
    }
}

#[derive(Debug)]
pub struct EntityKoch<New, Old: EntityItem, State>
where
    New: EntityItem + EntityKochFrom<Old, State>,
{
    pub from: EntityKochDb<Old>,
    pub state: RefCell<State>,
    pub total: u64,
    pub progress: u64,
    _new: PhantomData<New>,
}

impl<New, Old, State> EntityKoch<New, Old, State>
where
    New: EntityItem + EntityKochFrom<Old, State>,
    Old: EntityItem,
{
    pub fn new(from: EntityKochDb<Old>, state: State) -> Self {
        Self {
            progress: 0,
            total: from.total,
            from,
            state: RefCell::new(state),
            _new: PhantomData::<New>,
        }
    }

    pub fn get(&mut self, gene: &Gene) -> Result<New, ShahError> {
        let mut old = Old::default();
        self.from.get(gene, &mut old)?;
        New::entity_koch_from(old, self.state.borrow_mut())
    }
}

#[derive(Debug)]
pub struct EntityKochDb<T: EntityItem> {
    file: File,
    iteration: u16,
    total: u64,
    _e: PhantomData<T>,
    ls: String,
}

impl<T: EntityItem> EntityKochDb<T> {
    pub fn new(path: &str, iteration: u16) -> Result<Self, ShahError> {
        let path = Path::new("data/").join(path);
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

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
            total: 0,
            _e: PhantomData::<T>,
            ls: format!("<EntityKochDb {name}.{iteration}>"),
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

        self.check_head()?;

        self.total = (file_size - META_OFFSET) / T::N;

        Ok(())
    }

    fn check_head(&mut self) -> Result<(), ShahError> {
        let mut head = EntityHead::default();
        self.file.read_exact_at(head.as_binary_mut(), 0)?;

        if head.db_head.magic != ENTITY_MAGIC {
            log::error!(
                "{} invalid db magic: {:?} != {ENTITY_MAGIC:?}",
                self.ls,
                head.db_head.magic
            );
            return Err(DbError::InvalidDbHead)?;
        }
        if head.db_head.iteration != self.iteration {
            log::error!(
                "{} invalid {} != {}",
                self.ls,
                head.db_head.iteration,
                self.iteration
            );
            return Err(DbError::InvalidDbHead)?;
        }

        if head.item_size != T::N {
            log::error!(
                "{} head.item_size != current item size. {} != {}",
                self.ls,
                head.item_size,
                T::N
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        let schema = Schema::decode(&head.schema)?;
        if schema != T::shah_schema() {
            log::error!(
                "{} mismatch schema. did you forgot to update the iternation?",
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

        gene.check(entity.gene())?;

        Ok(())
    }
}
