use crate::error::{NotFound, ShahError, SystemError};
use crate::schema::{Schema, ShahSchema};
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
pub struct EntityDb<T: EntityDbItem, Old: EntityDbItem = T> {
    pub file: File,
    pub live: u64,
    pub dead_list: DeadList<GeneId, BLOCK_SIZE>,
    _e: PhantomData<T>,
    migration: Option<Box<EntityMigration<Old, T>>>,
}

#[derive(Debug)]
pub struct EntityMigration<Old: EntityDbItem, New: EntityDbItem> {
    pub db: EntityDb<Old, Old>,
    pub converter: fn(Old) -> New,
}

impl<T: EntityDbItem, Old: EntityDbItem> EntityDb<T, Old> {
    pub fn new(name: &str, iteration: u16) -> Result<Self, ShahError> {
        utils::validate_db_name(name)?;

        let mut path = Path::new("data/").join(name);
        std::fs::create_dir_all(&path)?;

        // let mut db_list = Vec::<(String, u16, DbHead, EntityMeta)>::new();
        // let current_schema = T::shah_schema().to_bytes();
        //
        // for item in path.read_dir()? {
        //     let item = item?;
        //     if !item.file_type()?.is_file() {
        //         continue;
        //     }
        //     let filename = item.file_name();
        //     let Some(filename) = filename.to_str() else { continue };
        //     let mut fns = filename.splitn(3, '.');
        //     let Some(oname) = fns.next() else { continue };
        //     let Some(iter) = fns.next() else { continue };
        //     let Some(ext) = fns.next() else { continue };
        //
        //     if oname != name || ext != ENTITY_EXT {
        //         continue;
        //     };
        //     let Ok(iter) = iter.parse::<u16>() else { continue };
        //
        //     let mut file = std::fs::OpenOptions::new()
        //         .read(true)
        //         .open(path.join(filename))?;
        //
        //     let file_size = file.seek(SeekFrom::End(0))?;
        //     if file_size < META_OFFSET {
        //         continue;
        //     }
        //
        //     let mut head = DbHead::default();
        //     file.read_exact_at(head.as_binary_mut(), 0)?;
        //     if head.magic != ENTITY_MAGIC {
        //         panic!("EntityDb<{filename}> magic does not match");
        //     }
        //     if iter != head.iteration {
        //         panic!("EntityDb<{filename}> iteration does not match its metadata");
        //     }
        //     let mut meta = EntityMeta::default();
        //     file.read_exact(meta.as_binary_mut())?;
        //     db_list.push((filename.to_string(), iter, head, meta));
        // }

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path.join(format!("{name}.{iteration}.shah")))?;

        let file_size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

        let mut head = DbHead::default();
        let mut schema = EntityMeta::default();

        if file_size < DbHead::N {
            head.magic = ENTITY_MAGIC;
            head.iteration = iteration;
            file.write_all(head.as_binary())?;
        } else {
            file.read_exact_at(head.as_binary_mut(), 0)?;
            if head.magic != ENTITY_MAGIC {
                log::error!(
                    "invalid db magic: {:?} != {ENTITY_MAGIC:?}",
                    head.magic
                );
                return Err(SystemError::InvalidDbHead)?;
            }
            if head.iteration != iteration {
                log::error!("invalid {} != {iteration}", head.iteration);
                return Err(SystemError::InvalidDbHead)?;
            }
        }

        if file_size < META_OFFSET {
            schema.item_size = T::N;
            // schema.schema = ;
            let svec = T::shah_schema().encode();
            schema.schema[0..svec.len()].clone_from_slice(&svec);
            file.write_all_at(schema.as_binary(), DbHead::N)?;
        } else {
            file.read_exact_at(schema.as_binary_mut(), DbHead::N)?;
            if schema.item_size != T::N {
                log::error!(
                    "schema.item_size != current item size. {} != {}",
                    schema.item_size,
                    T::N
                );
                return Err(SystemError::InvalidDbSchema)?;
            }

            let schema = Schema::decode(&schema.schema)?;
            if schema != T::shah_schema() {
                log::error!("mismatch current item schema vs db item schema. did you forgot to update the iternation?");
                return Err(SystemError::InvalidDbSchema)?;
            }
        }

        let db = Self {
            live: 0,
            dead_list: DeadList::<GeneId, BLOCK_SIZE>::new(),
            file,
            _e: PhantomData::<T>,
            migration: None, // migration: migration.map(|v| Box::new(v)),
        };

        Ok(db)
    }

    pub fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub fn setup<F>(mut self, mut f: F) -> Result<Self, ShahError>
    where
        F: FnMut(&mut Self, &T),
    {
        self.live = 0;
        self.dead_list.clear();
        let db_size = self.db_size()?;
        let mut entity = T::default();

        if db_size < T::N {
            self.file.seek(SeekFrom::Start(T::N - 1))?;
            self.file.write_all(&[0u8])?;
            return Ok(self);
        }

        if db_size == T::N {
            return Ok(self);
        }

        self.live = (db_size / T::N) - 1;
        return Ok(self);

        self.file.seek(SeekFrom::Start(T::N))?;
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

        let db_size = self.db_size()?;
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
        let db_size = self.db_size()?;
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
