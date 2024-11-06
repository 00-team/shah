use crate::error::SystemError;
use crate::{Binary, DeadList, Gene, GeneId, BLOCK_SIZE, PAGE_SIZE};
use std::{
    fmt::Debug,
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    os::unix::fs::FileExt,
};

const ITER_EXHAUSTION: u8 = 250;

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

    flag! {alive, set_alive}
    flag! {edited, set_edited}
    flag! {private, set_private}
}

#[derive(Debug)]
pub struct EntityDb<T>
where
    T: Default + Entity + Debug + Clone + Binary,
{
    pub file: File,
    pub live: u64,
    pub dead_list: DeadList<GeneId, BLOCK_SIZE>,
    _e: PhantomData<T>,
}

impl<T> EntityDb<T>
where
    T: Entity + Debug + Clone + Default + Binary,
{
    pub fn new(name: &str) -> Result<Self, SystemError> {
        std::fs::create_dir_all("data/")?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.bin"))?;

        let db = Self {
            live: 0,
            dead_list: DeadList::<GeneId, BLOCK_SIZE>::new(),
            file,
            _e: PhantomData::<T>,
        };

        Ok(db)
    }

    pub fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub fn setup(mut self) -> Result<Self, SystemError> {
        self.live = 0;
        self.dead_list.clear();
        let db_size = self.db_size()?;
        let mut entity = T::default();
        let buf = entity.as_binary_mut();

        if db_size < T::N {
            self.file.seek(SeekFrom::Start(T::N - 1))?;
            self.file.write_all(&[0u8])?;
            return Ok(self);
        }

        if db_size == T::N {
            return Ok(self);
        }

        self.live = (db_size / T::N) - 1;

        self.file.seek(SeekFrom::Start(T::N))?;
        loop {
            match self.file.read_exact(buf) {
                Ok(_) => {}
                Err(e) => match e.kind() {
                    ErrorKind::UnexpectedEof => break,
                    _ => Err(e)?,
                },
            }
            {
                let entity = T::from_binary(buf);
                if !entity.alive() {
                    log::debug!("dead: {entity:?}");
                    self.add_dead(entity.gene());
                }
            }
        }

        Ok(self)
    }

    pub fn seek_id(&mut self, id: GeneId) -> Result<(), SystemError> {
        if id == 0 {
            log::warn!("gene id is zero");
            return Err(SystemError::ZeroGeneId);
        }

        let db_size = self.db_size()?;
        let pos = id * T::N;

        if pos > db_size - T::N {
            log::warn!("invalid position: {pos}/{db_size}");
            return Err(SystemError::GeneIdNotInDatabase);
        }

        self.file.seek(SeekFrom::Start(pos))?;

        Ok(())
    }

    pub fn get(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), SystemError> {
        self.seek_id(gene.id)?;
        self.file.read_exact(entity.as_binary_mut())?;

        if !entity.alive() {
            return Err(SystemError::EntityNotAlive);
        }

        let og = entity.gene();

        if gene.pepper != og.pepper {
            log::warn!("bad gene {:?} != {:?}", gene.pepper, og.pepper);
            return Err(SystemError::BadGenePepper);
        }

        if gene.iter != og.iter {
            log::warn!("bad iter {:?} != {:?}", gene.iter, og.iter);
            return Err(SystemError::BadGeneIter);
        }

        Ok(())
    }

    pub fn seek_add(&mut self) -> Result<u64, SystemError> {
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

    pub fn new_gene(&mut self) -> Result<Gene, SystemError> {
        let mut gene = Gene { id: self.take_dead_id(), ..Default::default() };
        crate::utils::getrandom(&mut gene.pepper);
        gene.server = 69;
        gene.iter = 0;

        if gene.is_some() {
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

    pub fn add(&mut self, entity: &mut T) -> Result<(), SystemError> {
        entity.set_alive(true);
        if entity.gene().is_none() {
            entity.gene_mut().clone_from(&self.new_gene()?);
        }

        let id = entity.gene().id;
        self.file.write_all_at(entity.as_binary_mut(), id * T::N)?;
        self.live += 1;

        Ok(())
    }

    pub fn count(&mut self) -> Result<EntityCount, SystemError> {
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

    pub fn set(&mut self, entity: &T) -> Result<(), SystemError> {
        let mut old_entity = T::default();
        self.get(entity.gene(), &mut old_entity)?;
        self.file.seek_relative(-(T::N as i64))?;
        self.file.write_all(entity.as_binary())?;

        if !entity.alive() {
            self.add_dead(entity.gene());
        }

        Ok(())
    }

    pub fn list(
        &mut self, page: u64, result: &mut [T; PAGE_SIZE],
    ) -> Result<usize, SystemError> {
        self.seek_id(page * PAGE_SIZE as u64 + 1)?;
        let size = self.file.read(result.as_binary_mut())?;
        let count = size / T::S;
        if count != PAGE_SIZE {
            for item in result.iter_mut().skip(count) {
                item.as_binary_mut().fill(0)
            }
        }

        Ok(count)
    }
}
