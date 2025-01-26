use super::entity::{Entity, EntityCount, EntityDb};
use crate::error::{NotFound, ShahError, SystemError};
use crate::{
    Binary, DeadList, Gene, GeneId, BLOCK_SIZE, ITER_EXHAUSTION, PAGE_SIZE,
};
use std::fmt::Debug;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
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
    /// not iter exhausted slots.
    /// in other words slots that did not used all of their gene.iter
    pub empty: u8,
    _pad: [u8; 4],
}

#[derive(Debug)]
pub struct PondDb<T: Default + Entity + Debug + Clone + Copy + Binary + Duck> {
    pub file: File,
    pub live: u64,
    pub free_list: DeadList<Gene, BLOCK_SIZE>,
    pub index: EntityDb<Pond>,
    pub origins: EntityDb<Origin>,
    _e: PhantomData<T>,
}

impl<T: Default + Entity + Debug + Clone + Copy + Binary + Duck> PondDb<T> {
    pub fn new(name: &str) -> Result<Self, ShahError> {
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

    pub fn setup(mut self) -> Result<Self, ShahError> {
        self.live = 0;
        self.free_list.clear();

        self.origins = self.origins.setup(|_, _| {})?;
        self.index = self.index.setup(|_, pond| {
            if pond.is_alive() && pond.is_free() && pond.empty > 0 {
                self.free_list.push(pond.gene);
                if pond.next.is_some() || pond.past.is_some() {
                    log::warn!("invalid pond: {pond:?}");
                }
            }
        })?;

        // let db_size = self.db_size()?;
        // if db_size < T::N {
        //     self.file.seek(SeekFrom::Start(T::N - 1))?;
        //     self.file.write_all(&[0u8])?;
        // }

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
                self.live -= 1;
            }
        }

        Ok(self)
    }

    pub fn take_free(&mut self) -> Option<Gene> {
        self.free_list.pop(|_| true)
    }

    fn add_empty_pond(
        &mut self, origin: &mut Origin, mut pond: Pond,
    ) -> Result<(), ShahError> {
        origin.ponds -= 1;
        let mut old_pond = Pond::default();
        let mut buf = [T::default(); PAGE_SIZE];
        self.seek_id(pond.stack)?;
        self.file.read_exact(buf.as_binary_mut())?;
        pond.empty = 0;
        pond.alive = 0;
        for item in buf {
            if item.gene().iter < ITER_EXHAUSTION {
                pond.empty += 1;
            }
            if item.is_alive() {
                log::warn!("adding a non-free pond to free_list");
                return Ok(());
            }
        }

        if origin.first == pond.gene {
            origin.first = pond.next;
        }

        if origin.last == pond.gene {
            origin.last = pond.past;
        }

        if pond.past.is_some() {
            self.index.get(&pond.past, &mut old_pond)?;
            old_pond.next = pond.next;
            self.index.set(&old_pond)?;
        }

        if pond.next.is_some() {
            self.index.get(&pond.next, &mut old_pond)?;
            old_pond.past = pond.past;
            self.index.set(&old_pond)?;
        }

        pond.next.zeroed();
        pond.past.zeroed();
        pond.origin.zeroed();
        pond.set_free(true);
        self.index.set(&pond)?;
        self.free_list.push(pond.gene);
        Ok(())
    }

    fn half_empty_pond(
        &mut self, origin: &mut Origin,
    ) -> Result<Pond, ShahError> {
        let mut pond_gene = origin.first;
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

                if pond.is_alive() {
                    pond.next = new.gene;
                    new.past = origin.last;
                    origin.last = new.gene;
                    self.index.set(&pond)?;
                } else {
                    new.past.zeroed();
                    origin.first = new.gene;
                    origin.last = new.gene;
                }
                return Ok(new);
            }

            self.index.get(&pond_gene, &mut pond)?;
            if pond.empty > 0 {
                return Ok(pond);
            }
            pond_gene = pond.next;
        }
    }

    pub fn add(
        &mut self, origene: &Gene, item: &mut T,
    ) -> Result<(), ShahError> {
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

        let stack = if pond.stack == 0 {
            let pos = self.seek_add()?;
            let stack = pos / T::N;
            ig.id = stack;
            ig.iter = 0;
            buf[0] = *item;

            pond.stack = stack;
            pond.empty = PAGE_SIZE as u8 - 1;
            stack
        } else {
            self.seek_id(pond.stack)?;
            self.file.read_exact(buf.as_binary_mut())?;
            let mut found_empty_slot = false;
            for (x, slot) in buf.iter_mut().enumerate() {
                let sg = slot.gene();
                if !slot.is_alive() && sg.iter < ITER_EXHAUSTION {
                    let ig = item.gene_mut();
                    ig.id = pond.stack + x as u64;
                    ig.iter = if sg.id != 0 { sg.iter + 1 } else { 0 };
                    *slot = *item;
                    found_empty_slot = true;
                    pond.empty -= 1;
                    break;
                }
            }
            if !found_empty_slot {
                log::error!("could not found an empty slot for item");
            }

            pond.stack
        };

        self.live += 1;
        self.file.write_all_at(buf.as_binary(), stack * T::N)?;
        self.index.set(&pond)?;
        self.origins.set(&origin)?;

        Ok(())
    }

    pub fn seek_add(&mut self) -> Result<u64, ShahError> {
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

    pub fn seek_id(&mut self, id: GeneId) -> Result<(), ShahError> {
        if id == 0 {
            log::warn!("gene id is zero");
            return Err(NotFound::ZeroGeneId)?;
        }

        let db_size = self.db_size()?;
        let pos = id * T::N;

        if db_size < T::N || pos > db_size - T::N {
            log::warn!("invalid position: {pos}/{db_size}");
            return Err(NotFound::GeneIdNotInDatabase)?;
        }

        let rs = self.file.seek(SeekFrom::Start(pos))?;
        assert_eq!(rs, pos, "could not seek correctly");

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
        if gene.id != og.id {
            log::error!("invalid gene.id != og.id: {} != {}", gene.id, og.id);
            return Err(SystemError::MismatchGeneId)?;
        }

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

    pub fn count(&mut self) -> Result<EntityCount, ShahError> {
        let db_size = self.db_size()?;
        let total = db_size / T::N - 1;
        Ok(EntityCount { total, alive: self.live })
    }

    pub fn set(&mut self, entity: &mut T) -> Result<(), ShahError> {
        if !entity.is_alive() {
            return Err(NotFound::DeadSet)?;
        }

        let mut old_entity = T::default();
        self.get(entity.gene(), &mut old_entity)?;

        *entity.pond_mut() = *old_entity.pond();

        self.file.seek_relative(-(T::N as i64))?;
        self.file.write_all(entity.as_binary())?;

        Ok(())
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        self.get(gene, entity)?;

        entity.set_alive(false);

        self.live -= 1;

        self.file.seek_relative(-(T::N as i64))?;
        self.file.write_all(entity.as_binary())?;

        let mut pond = Pond::default();
        let mut origin = Origin::default();

        self.index.get(entity.pond(), &mut pond)?;
        pond.alive -= 1;

        self.origins.get(&pond.origin, &mut origin)?;
        origin.items -= 1;

        if pond.alive == 0 {
            self.add_empty_pond(&mut origin, pond)?;
        } else {
            self.index.set(&pond)?;
        }

        self.origins.set(&origin)?;

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

    pub fn pond_list(
        &mut self, pond: &mut Pond, result: &mut [T; PAGE_SIZE],
    ) -> Result<(), ShahError> {
        let pond_gene = pond.gene;
        self.index.get(&pond_gene, pond)?;

        self.seek_id(pond.stack)?;
        self.file.read_exact(result.as_binary_mut())?;

        Ok(())
    }

    pub fn pond_free(&mut self, pond: &mut Pond) -> Result<(), ShahError> {
        let mut buf = [T::default(); PAGE_SIZE];

        self.seek_id(pond.stack)?;
        self.file.read_exact(buf.as_binary_mut())?;

        pond.empty = 0;
        for item in buf.iter_mut() {
            if item.is_alive() {
                item.set_alive(false);
                self.live -= 1;
            }
            if item.gene().iter < ITER_EXHAUSTION {
                pond.empty += 1;
            }
        }

        self.seek_id(pond.stack)?;
        self.file.write_all(buf.as_binary())?;

        pond.set_free(true);
        pond.alive = 0;

        self.index.set(pond)?;
        self.free_list.push(pond.gene);

        Ok(())
    }

    pub fn cascade(&mut self, origene: &Gene) -> Result<(), ShahError> {
        let mut origin = Origin::default();
        self.origins.get(origene, &mut origin)?;

        let mut pond_gene = origin.first;
        let mut pond = Pond::default();
        loop {
            if pond_gene.is_none() {
                break;
            }

            self.index.get(&pond_gene, &mut pond)?;
            pond_gene = pond.next;
            self.pond_free(&mut pond)?;
        }

        self.origins.del(origene, &mut origin)?;

        Ok(())
    }
}
