use super::entity::{Entity, EntityCount, EntityDb};
use crate::error::SystemError;
use crate::{
    Binary, DeadList, Gene, GeneId, BLOCK_SIZE, ITER_EXHAUSTION, PAGE_SIZE,
};
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::marker::PhantomData;
use std::os::unix::fs::FileExt;

// NOTE's for sorted ponds.
// 1. we dont need to sort items in each "stack" and we can just leave that
//    for frontend. just have a min/max value in each "pond" and
//    if item > pond.max then move it to pond.past ...

#[crate::model]
#[derive(Debug, Clone, Copy, crate::Entity)]
pub struct Origin {
    pub gene: Gene,
    pub owner: Gene,
    pub ponds: u64,
    pub items: u64,
    pub first: Gene,
    pub last: Gene,
    #[entity_flags]
    pub entity_flags: u8,
    pub _pad: [u8; 7],
}

pub trait Duck {
    fn pond(&self) -> &Gene;
    fn pond_mut(&mut self) -> &mut Gene;
}

#[crate::model]
#[derive(Debug, crate::Entity, Clone)]
pub struct Pond {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub origin: Gene,
    pub stack: GeneId,
    #[entity_flags]
    pub entity_flags: u8,
    #[flags(free)]
    pub flags: u8,
    pub alive: u8,
    _pad: [u8; 5],
}

#[derive(Debug)]
pub struct PondDb<T: Duck> {
    pub file: File,
    pub live: u64,
    pub free_list: DeadList<Gene, BLOCK_SIZE>,
    pub index: EntityDb<Pond>,
    pub origins: EntityDb<Origin>,
    _e: PhantomData<T>,
}

impl<T: Default + Entity + Debug + Clone + Copy + Binary + Duck> PondDb<T> {
    pub fn new(name: &str) -> Result<Self, SystemError> {
        std::fs::create_dir_all("data/")?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.pond.items.bin"))?;

        let db = Self {
            file,
            live: 0,
            free_list: DeadList::<Gene, BLOCK_SIZE>::new(),
            index: EntityDb::<Pond>::new(&format!("{name}.pond.index"))?,
            // items: EntityDb::<Brood<T>>::new(&format!("{name}.pond.brood"))?,
            origins: EntityDb::<Origin>::new(&format!("{name}.pond.origin"))?,
            _e: PhantomData,
        };

        Ok(db)
    }

    pub fn setup(mut self) -> Result<Self, SystemError> {
        self.origins = self.origins.setup(|_, _| {})?;
        self.index = self.index.setup(|_, pond| {
            if pond.alive() && pond.free() {
                self.free_list.push(pond.gene);
                if pond.next.is_some() || pond.past.is_some() {
                    log::warn!("invalid pond: {pond:?}");
                }
            }
        })?;

        Ok(self)
    }

    pub fn take_free(&mut self) -> Option<Gene> {
        self.free_list.pop(|_| true)
    }

    pub fn half_empty_pond(
        &mut self, origin: &mut Origin,
    ) -> Result<Pond, SystemError> {
        let mut pond_gene = origin.first;
        let mut past_pond = Pond::default();
        let mut pond = Pond::default();
        loop {
            if pond_gene.is_none() {
                let mut new = Pond::default();
                if let Some(free) = self.take_free() {
                    self.index.get(&free, &mut new)?;
                } else {
                    self.index.add(&mut new)?;
                }
                new.next.zeroed();
                new.alive = 0;
                new.origin = origin.gene;
                new.set_free(false);

                origin.ponds += 1;

                if past_pond.alive() {
                    past_pond.next = new.gene;
                    new.past = origin.last;
                    origin.last = new.gene;
                } else {
                    new.past.zeroed();
                    origin.first = new.gene;
                    origin.last = new.gene;
                }
                return Ok(new);
            }

            past_pond = pond.clone();
            self.index.get(&pond_gene, &mut pond)?;
            if pond.alive < PAGE_SIZE as u8 {
                return Ok(pond);
            }
            pond_gene = pond.next;
        }
    }

    pub fn add(
        &mut self, origene: &Gene, item: &mut T,
    ) -> Result<(), SystemError> {
        item.set_alive(true);

        let mut origin = Origin::default();
        self.origins.get(origene, &mut origin)?;
        origin.items += 1;

        let mut pond = self.half_empty_pond(&mut origin)?;
        pond.alive += 1;

        let mut buf = [T::default(); PAGE_SIZE];
        *item.pond_mut() = pond.gene;
        let ig = item.gene_mut();
        ig.server = 69;
        crate::utils::getrandom(&mut ig.pepper);

        let pos = if pond.stack == 0 {
            let pos = self.seek_add()?;
            ig.id = pos / T::N;
            ig.iter = 0;
            buf[0] = *item;

            pond.stack = pos;
            pos
        } else {
            let pos = pond.stack;
            self.file.read_exact_at(buf.as_binary_mut(), pos)?;
            for (x, slot) in buf.iter_mut().enumerate() {
                let sg = slot.gene();
                if !slot.alive() && sg.iter < ITER_EXHAUSTION {
                    let ig = item.gene_mut();
                    ig.id = pos / T::N + x as u64;
                    ig.iter = if sg.id != 0 { sg.iter + 1 } else { 0 };
                    *slot = *item;
                }
            }

            pos
        };

        self.file.write_all_at(buf.as_binary(), pos)?;
        self.index.set(&pond)?;
        self.origins.set(&origin)?;

        Ok(())
    }

    pub fn seek_add(&mut self) -> Result<u64, SystemError> {
        let pos = self.file.seek(SeekFrom::End(0))?;
        if pos == 0 {
            self.file.seek(SeekFrom::Start(T::N))?;
            return Ok(T::N);
        }

        let offset = (pos % (T::N * PAGE_SIZE as u64)) as i64;
        let offset = offset - T::N as i64;
        if offset != 0 {
            log::warn!("seek_add bad offset: {}", offset);
            return Ok(self.file.seek(SeekFrom::Current(-offset))?);
        }

        Ok(pos)
    }

    pub fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
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

    pub fn count(&mut self) -> Result<EntityCount, SystemError> {
        let db_size = self.db_size()?;
        let total = db_size / T::N - 1;
        Ok(EntityCount { total, alive: self.live })
    }

    pub fn set(&mut self, entity: &T) -> Result<(), SystemError> {
        if !entity.alive() {
            return Err(SystemError::DeadSet);
        }

        let mut old_entity = T::default();
        self.get(entity.gene(), &mut old_entity)?;
        self.file.seek_relative(-(T::N as i64))?;
        self.file.write_all(entity.as_binary())?;

        Ok(())
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), SystemError> {
        self.get(gene, entity)?;
        self.file.seek_relative(-(T::N as i64))?;
        entity.set_alive(false);
        self.file.write_all(entity.as_binary())?;

        let mut pond = Pond::default();
        self.index.get(entity.pond(), &mut pond)?;
        pond.alive -= 1;
        // TODO: delete pond

        if gene.iter < ITER_EXHAUSTION {
            self.free_list.push(*gene);
        }

        Ok(())
    }
}
