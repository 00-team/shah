use std::{
    fmt::Debug,
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    os::unix::fs::FileExt,
    // slice::{from_raw_parts, from_raw_parts_mut},
};

use crate::error::SystemError;
use crate::{Binary, Gene, GeneId};

pub const PAGE_SIZE: usize = 32;
// const REQUEST_SIZE: u16 = 40960; // 4096 * 10
// const SNAKE_MAX_LENGTH: u16 = 32768; // 4096 * 8
const ITER_EXHAUSTION: u8 = 250;

pub struct EntityMetadata {
    pub version: u32,
    pub size: u32,
    pub fields: String,
}

#[repr(u8)]
pub enum EntityFlag {
    ALIVE = (1 << 0),
    EDITED = (1 << 1),
    PRIVATE = (1 << 2),
}

pub struct EntityCount {
    pub alive: u64,
    pub total: u64,
}

macro_rules! flag {
    ($flag:path, $name:ident, $set:ident) => {
        fn $name(&self) -> bool {
            self.get_flag($flag as u8)
        }
        fn $set(&mut self, $name: bool) -> &mut Self {
            self.set_flag($flag as u8, $name)
        }
    };
}

pub trait Entity {
    fn gene(&self) -> &Gene;
    fn flags(&self) -> &u8;

    fn gene_mut(&mut self) -> &mut Gene;
    fn flags_mut(&mut self) -> &mut u8;

    fn get_flag(&self, flag: u8) -> bool {
        (*self.flags() & flag) == flag
    }

    fn set_flag(&mut self, flag: u8, value: bool) -> &mut Self {
        if value {
            *self.flags_mut() |= flag;
        } else {
            *self.flags_mut() &= !flag;
        }
        self
    }

    flag! {EntityFlag::ALIVE, alive, set_alive}
    flag! {EntityFlag::EDITED, edited, set_edited}
    flag! {EntityFlag::PRIVATE, private, set_private}
}

#[derive(Debug)]
pub struct EntityDb<T>
where
    T: Default + Entity + Debug + Clone + Binary,
{
    pub file: File,
    _e: PhantomData<T>,
    pub live: u64,
    pub dead: u64,
    pub dead_list: [GeneId; 4096],
}

impl<T> EntityDb<T>
where
    T: Entity + Debug + Clone + Default + Binary,
{
    pub fn new(name: &'static str) -> Result<Self, SystemError> {
        std::fs::create_dir_all("data/")?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.bin"))?;

        let db = Self {
            live: 0,
            dead: 0,
            dead_list: [0; 4096],
            file,
            _e: PhantomData::<T>,
        };

        Ok(db)
    }

    pub fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub fn update_population(&mut self) -> Result<(), SystemError> {
        // let db = self.db();
        self.live = 0;
        self.dead = 0;
        self.dead_list.fill(0);
        let db_size = self.db_size()?;
        let mut entity = T::default();
        let buf = entity.as_binary_mut();

        if db_size < T::N {
            self.file.seek(SeekFrom::Start(T::N - 1))?;
            self.file.write_all(&[0u8])?;
            return Ok(());
        }

        if db_size == T::N {
            return Ok(());
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
                log::info!("entity: {entity:?}");
                if !entity.alive() {
                    log::info!("dead: {entity:?}");
                    self.add_dead(entity.gene());
                }
            }
        }

        Ok(())
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

    pub fn add(&mut self, entity: &mut T) -> Result<(), SystemError> {
        entity.set_alive(true);
        if entity.gene().id == 0 {
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
        if self.dead == 0 {
            return 0;
        }

        for dead in self.dead_list.iter_mut() {
            if *dead != 0 {
                let id = *dead;
                *dead = 0;
                self.dead -= 1;
                return id;
            }
        }

        log::warn!("invalid state of dead list");
        self.dead = 0;
        0
    }

    pub fn add_dead(&mut self, gene: &Gene) {
        self.live -= 1;
        if self.dead as usize >= self.dead_list.len()
            || gene.iter >= ITER_EXHAUSTION
        {
            return;
        }

        let mut set = false;
        for slot in self.dead_list.iter_mut() {
            if *slot == gene.id {
                log::warn!("{gene:?} already exists in dead list");
                return;
            }

            if !set && *slot == 0 {
                *slot = gene.id;
                set = true;
                self.dead += 1;
            }
        }
    }

    pub fn set(&mut self, entity: &mut T) -> Result<(), SystemError> {
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
