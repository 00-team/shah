use crate::error::{DbError, NotFound, ShahError};
use crate::schema::{Schema, ShahSchema};
use crate::state::Task;
use crate::{
    utils, Binary, DbHead, DeadList, Gene, GeneId, ShahMagic, ShahMagicDb,
    BLOCK_SIZE, ITER_EXHAUSTION, PAGE_SIZE,
};
use std::path::Path;
use std::{
    fmt::Debug,
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    os::unix::fs::FileExt,
};

const META_OFFSET: u64 = DbHead::N + EntityMeta::N;
const ENTITY_MAGIC: ShahMagic =
    ShahMagic::new_const(ShahMagicDb::Entity as u16);

#[crate::model]
struct EntityMeta {
    item_size: u64,
    schema: [u8; 4096],
}

#[derive(Debug)]
pub struct EntityCount {
    pub alive: u64,
    pub total: u64,
}

macro_rules! flag {
    ($name:ident, $set:ident) => {
        fn $name(&self) -> bool;
        fn $set(&mut self, $name: bool) -> &mut Self;
    };
}

pub trait Entity {
    fn gene(&self) -> &Gene;
    fn gene_mut(&mut self) -> &mut Gene;

    flag! {is_alive, set_alive}
    flag! {is_edited, set_edited}
    flag! {is_private, set_private}
}

pub trait EntityDbItem:
    Default + Entity + Debug + Clone + Binary + ShahSchema
{
}
impl<T: Default + Entity + Debug + Clone + Binary + ShahSchema> EntityDbItem
    for T
{
}

#[derive(Debug)]
pub struct EntityDb<'a, T: EntityDbItem, Old: EntityDbItem = T, State = ()>
where
    T: EntityMigrateFrom<State, Old>,
{
    pub file: File,
    pub live: u64,
    pub dead_list: DeadList<GeneId, BLOCK_SIZE>,
    migration: Option<Box<EntityMigration<'a, Old, State>>>,
    _e: PhantomData<T>,
}

pub trait EntityMigrateFrom<State, Old: EntityDbItem> {
    fn entity_migrate_from(state: &mut State, old: Old) -> Self;
}
impl<State, T: EntityDbItem> EntityMigrateFrom<State, T> for T {
    fn entity_migrate_from(_: &mut State, old: T) -> Self {
        old
    }
}

#[derive(Debug)]
pub struct EntityMigration<'a, Old: EntityDbItem, State> {
    pub from: EntityDb<'static, Old>,
    pub state: &'a mut State,
}

struct EntitySetupTask {}
impl Task for EntitySetupTask {
    fn work(&mut self) {}
}

struct EntityMigrateTask {}
impl Task for EntityMigrateTask {
    fn work(&mut self) {}
}

impl<'a, State, T: EntityDbItem, Old: EntityDbItem> EntityDb<'a, T, Old, State>
where
    T: EntityMigrateFrom<State, Old>,
{
    pub fn new(path: &str, iteration: u16) -> Result<Self, ShahError> {
        let path = Path::new("data/").join(path);
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path: {path}");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&path)?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.join(format!("{name}.{iteration}.shah")))?;

        let mut db = Self {
            live: 0,
            dead_list: DeadList::<GeneId, BLOCK_SIZE>::new(),
            file,
            _e: PhantomData::<T>,
            migration: None,
        };

        db.check_db_head(iteration)?;
        db.check_schema()?;

        Ok(db)
    }

    fn check_db_head(&mut self, iteration: u16) -> Result<(), ShahError> {
        let db_size = self.file_size()?;
        self.file.seek(SeekFrom::Start(0))?;

        let mut head = DbHead::default();

        if db_size < DbHead::N {
            head.magic = ENTITY_MAGIC;
            head.iteration = iteration;
            self.file.write_all(head.as_binary())?;
        } else {
            self.file.read_exact_at(head.as_binary_mut(), 0)?;
            if head.magic != ENTITY_MAGIC {
                log::error!(
                    "invalid db magic: {:?} != {ENTITY_MAGIC:?}",
                    head.magic
                );
                return Err(DbError::InvalidDbHead)?;
            }
            if head.iteration != iteration {
                log::error!("invalid {} != {iteration}", head.iteration);
                return Err(DbError::InvalidDbHead)?;
            }
        }

        Ok(())
    }

    fn check_schema(&mut self) -> Result<(), ShahError> {
        let db_size = self.file_size()?;
        let mut schema = EntityMeta::default();

        if db_size < DbHead::N + EntityMeta::N {
            schema.item_size = T::N;
            let svec = T::shah_schema().encode();
            schema.schema[0..svec.len()].clone_from_slice(&svec);
            self.file.write_all_at(schema.as_binary(), DbHead::N)?;
        } else {
            self.file.read_exact_at(schema.as_binary_mut(), DbHead::N)?;
            if schema.item_size != T::N {
                log::error!(
                    "schema.item_size != current item size. {} != {}",
                    schema.item_size,
                    T::N
                );
                return Err(DbError::InvalidDbSchema)?;
            }

            let schema = Schema::decode(&schema.schema)?;
            if schema != T::shah_schema() {
                log::error!("mismatch current item schema vs db item schema. did you forgot to update the iternation?");
                return Err(DbError::InvalidDbSchema)?;
            }
        }

        Ok(())
    }

    pub fn set_migration(
        &mut self, migration: EntityMigration<'a, Old, State>,
    ) {
        self.migration = Some(Box::new(migration));
    }

    pub fn tasks(&mut self) -> Vec<Box<dyn Task>> {
        vec![Box::new(EntitySetupTask {}), Box::new(EntityMigrateTask {})]
    }

    pub fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub fn setup<F>(mut self, mut f: F) -> Result<Self, ShahError>
    where
        F: FnMut(&mut Self, &T),
    {
        self.live = 0;
        self.dead_list.clear();
        let file_size = self.file_size()?;
        if file_size < META_OFFSET {
            return Err(DbError::BadInit)?;
        }

        let mut entity = T::default();

        if file_size < META_OFFSET + T::N {
            self.file.seek(SeekFrom::Start(META_OFFSET + T::N - 1))?;
            self.file.write_all(&[0u8])?;
            return Ok(self);
        }

        if file_size == META_OFFSET + T::N {
            return Ok(self);
        }

        self.live = ((file_size - META_OFFSET) / T::N) - 1;
        // return Ok(self);

        self.file.seek(SeekFrom::Start(META_OFFSET + T::N))?;
        loop {
            match self.file.read_exact(entity.as_binary_mut()) {
                Ok(_) => {}
                Err(e) => match e.kind() {
                    ErrorKind::UnexpectedEof => break,
                    _ => Err(e)?,
                },
            }

            if !entity.is_alive() {
                let gene = entity.gene();
                log::debug!("dead entity: {entity:?}");
                self.add_dead(gene);
            }

            f(&mut self, &entity);
        }

        Ok(self)
    }

    pub fn seek_id(&mut self, id: GeneId) -> Result<(), ShahError> {
        if id == 0 {
            log::warn!("gene id is zero");
            return Err(NotFound::ZeroGeneId)?;
        }

        let db_size = self.file_size()?;
        let pos = id * T::N;

        if pos > db_size - T::N {
            log::warn!("invalid position: {pos}/{db_size}");
            return Err(NotFound::GeneIdNotInDatabase)?;
        }

        self.file.seek(SeekFrom::Start(pos))?;

        Ok(())
    }

    pub fn get(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        self.seek_id(gene.id)?;
        self.file.read_exact(entity.as_binary_mut())?;

        if !entity.is_alive() {
            return Err(NotFound::EntityNotAlive)?;
        }

        let og = entity.gene();

        if gene.pepper != og.pepper {
            log::warn!("bad gene {:?} != {:?}", gene.pepper, og.pepper);
            return Err(NotFound::BadGenePepper)?;
        }

        if gene.iter != og.iter {
            log::warn!("bad iter {:?} != {:?}", gene.iter, og.iter);
            return Err(NotFound::BadGeneIter)?;
        }

        Ok(())
    }

    pub fn seek_add(&mut self) -> Result<u64, ShahError> {
        let pos = self.file.seek(SeekFrom::End(0))?;
        if pos == 0 {
            self.file.seek(SeekFrom::Start(T::N))?;
            return Ok(T::N);
        }

        let offset = pos % T::N;
        if offset != 0 {
            log::warn!(
                "{}:{} seek_add bad offset: {}",
                file!(),
                line!(),
                offset
            );
            return Ok(self.file.seek(SeekFrom::Current(-(offset as i64)))?);
        }

        Ok(pos)
    }

    pub fn new_gene(&mut self) -> Result<Gene, ShahError> {
        let mut gene = Gene { id: self.take_dead_id(), ..Default::default() };
        crate::utils::getrandom(&mut gene.pepper);
        gene.server = 69;
        gene.iter = 0;

        if gene.id != 0 {
            let mut og = Gene::default();
            // let mut og = [0u8; size_of::<Gene>()];
            self.file.read_exact_at(og.as_binary_mut(), gene.id * T::N)?;
            if og.iter >= ITER_EXHAUSTION {
                gene.id = self.seek_add()? / T::N;
            } else {
                gene.iter = og.iter + 1;
                self.file.seek(SeekFrom::Current(-(Gene::N as i64)))?;
            }
        } else {
            gene.id = self.seek_add()? / T::N;
        }

        Ok(gene)
    }

    pub fn add(&mut self, entity: &mut T) -> Result<(), ShahError> {
        entity.set_alive(true);
        if entity.gene().id == 0 {
            entity.gene_mut().clone_from(&self.new_gene()?);
        }

        let id = entity.gene().id;
        self.file.write_all_at(entity.as_binary_mut(), id * T::N)?;
        self.live += 1;

        Ok(())
    }

    pub fn count(&mut self) -> Result<EntityCount, ShahError> {
        let db_size = self.file_size()?;
        let total = db_size / T::N - 1;
        Ok(EntityCount { total, alive: self.live })
    }

    pub fn take_dead_id(&mut self) -> GeneId {
        self.dead_list.pop(|_| true).unwrap_or_default()
    }

    pub fn add_dead(&mut self, gene: &Gene) {
        self.live -= 1;

        if gene.iter >= ITER_EXHAUSTION {
            return;
        }

        self.dead_list.push(gene.id);
    }

    pub fn set(&mut self, entity: &T) -> Result<(), ShahError> {
        if !entity.is_alive() {
            return Err(NotFound::DeadSet)?;
        }

        let mut old_entity = T::default();
        self.get(entity.gene(), &mut old_entity)?;
        self.file.seek_relative(-(T::N as i64))?;
        self.file.write_all(entity.as_binary())?;

        Ok(())
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        self.get(gene, entity)?;
        self.file.seek_relative(-(T::N as i64))?;
        entity.set_alive(false);
        self.file.write_all(entity.as_binary())?;

        self.add_dead(gene);

        Ok(())
    }

    pub fn list(
        &mut self, page: u64, result: &mut [T; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        self.seek_id(page * PAGE_SIZE as u64 + 1)?;
        let size = self.file.read(result.as_binary_mut())?;
        let count = size / T::S;
        if count != PAGE_SIZE {
            for item in result.iter_mut().skip(count) {
                item.zeroed()
            }
        }

        Ok(count)
    }
}
