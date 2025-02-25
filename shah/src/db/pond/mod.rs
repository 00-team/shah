use super::entity::{
    Entity, EntityCount, EntityDb, EntityItem, EntityKochFrom, ENTITY_META,
};
use crate::models::task_list::{Performed, Task, TaskList};
use crate::models::{Binary, DeadList, Gene, GeneId};
use crate::{utils, BLOCK_SIZE, ITER_EXHAUSTION, PAGE_SIZE};
use crate::{IsNotFound, NotFound, ShahError};

use std::fmt::Debug;
use std::path::Path;

// NOTE's for sorted ponds.
// 1. we dont need to sort items in each "stack" and we can just leave that
//    for frontend. just have a min/max value in each "pond" and
//    if item > pond.max then move it to pond.past ...

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, Clone, crate::Entity)]
pub struct Origin {
    pub gene: Gene,
    pub owner: Gene,
    pub ponds: u64,
    pub items: u64,
    pub first: Gene,
    pub last: Gene,
    pub entity_flags: u8,
    pub _pad: [u8; 7],
    growth: u64,
}

pub trait Duck {
    fn pond(&self) -> &Gene;
    fn pond_mut(&mut self) -> &mut Gene;
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, crate::Entity, Clone)]
pub struct Pond {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub origin: Gene,
    pub stack: GeneId,
    pub growth: u64,
    pub entity_flags: u8,
    #[flags(free)]
    pub flags: u8,
    pub alive: u8,
    /// not iter exhausted slots.
    /// in other words slots that did not used all of their gene.iter
    pub empty: u8,
    _pad: [u8; 4],
}
type PondIndexDb = EntityDb<Pond>;
type OriginDb = EntityDb<Origin>;

pub trait PondItem: EntityItem + Duck + Copy {}
impl<T: EntityItem + Duck + Copy> PondItem for T {}

#[derive(Debug)]
pub struct PondDb<T: PondItem + EntityKochFrom<O, S>, O: EntityItem = T, S = ()>
{
    pub free_list: DeadList<Gene, BLOCK_SIZE>,
    pub index: PondIndexDb,
    pub origins: OriginDb,
    pub ls: String,
    items: EntityDb<T, O, S>,
    tasks: TaskList<3, Task<Self>>,
}

impl<T: PondItem + EntityKochFrom<O, S>, O: EntityItem, S> PondDb<T, O, S> {
    pub fn new(path: &str, revision: u16) -> Result<Self, ShahError> {
        let data_path = Path::new("data/").join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let db = Self {
            free_list: DeadList::<Gene, BLOCK_SIZE>::new(),
            index: PondIndexDb::new(&format!("{path}/index"), 0)?,
            origins: OriginDb::new(&format!("{path}/origin"), 0)?,
            items: EntityDb::<T, O, S>::new(path, revision)?,
            tasks: TaskList::new([
                Self::work_index,
                Self::work_origins,
                Self::work_items,
            ]),
            ls: format!("<PondDb {name}.{revision} />"),
        };

        Ok(db)
    }

    fn work_items(&mut self) -> Result<Performed, ShahError> {
        self.items.work()
    }

    fn work_index(&mut self) -> Result<Performed, ShahError> {
        self.index.work()
    }

    fn work_origins(&mut self) -> Result<Performed, ShahError> {
        self.origins.work()
    }

    pub fn work(&mut self) -> Result<Performed, ShahError> {
        self.tasks.start();
        while let Some(task) = self.tasks.next() {
            if task(self)?.0 {
                return Ok(Performed(true));
            }
        }
        Ok(Performed(false))
    }

    pub fn take_free(&mut self) -> Option<Gene> {
        self.free_list.pop(|_| true)
    }

    fn add_empty_pond(
        &mut self, origin: &mut Origin, mut pond: Pond,
    ) -> Result<(), ShahError> {
        if origin.ponds > 0 {
            origin.ponds -= 1;
        }
        let mut old_pond = Pond::default();

        let mut buf = [T::default(); PAGE_SIZE];
        self.items.read_buf_at(&mut buf, pond.stack)?;

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

        if let Err(e) = self.index.get(&pond.past, &mut old_pond) {
            e.not_found_ok()?;
        } else {
            old_pond.next = pond.next;
            self.index.set(&mut old_pond)?;
        }

        if let Err(e) = self.index.get(&pond.next, &mut old_pond) {
            e.not_found_ok()?;
        } else {
            old_pond.past = pond.past;
            self.index.set(&mut old_pond)?;
        }

        pond.next.zeroed();
        pond.past.zeroed();
        pond.origin.zeroed();
        pond.set_free(true);
        self.index.set(&mut pond)?;
        self.free_list.push(pond.gene);
        Ok(())
    }

    fn half_empty_pond(
        &mut self, origin: &mut Origin,
    ) -> Result<Pond, ShahError> {
        let mut pond_gene = origin.first;
        let mut pond = Pond::default();
        loop {
            if let Err(e) = self.index.get(&pond_gene, &mut pond) {
                e.not_found_ok()?;

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
                    self.index.set(&mut pond)?;
                } else {
                    new.past.zeroed();
                    origin.first = new.gene;
                    origin.last = new.gene;
                }
                return Ok(new);
            }

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
            let stack = self.new_stack_id()?;
            ig.id = stack;
            ig.iter = 0;
            buf[0] = *item;

            pond.stack = stack;
            pond.empty = PAGE_SIZE as u8 - 1;
            stack
        } else {
            self.items.read_buf_at(&mut buf, pond.stack)?;

            let mut found_empty_slot = false;
            for (x, slot) in buf.iter_mut().enumerate() {
                let sg = slot.gene();
                if !slot.is_alive() && sg.iter < ITER_EXHAUSTION {
                    let ig = item.gene_mut();
                    ig.id = pond.stack + x as u64;
                    ig.iter = if sg.id != 0 { sg.iter + 1 } else { 0 };
                    *slot = *item;
                    found_empty_slot = true;
                    if pond.empty > 0 {
                        pond.empty -= 1;
                    }
                    break;
                }
            }
            if !found_empty_slot {
                log::error!("could not found an empty slot for item");
            }

            pond.stack
        };

        self.items.write_buf_at(&buf, stack)?;
        self.index.set(&mut pond)?;
        self.origins.set(&mut origin)?;

        Ok(())
    }

    pub fn new_stack_id(&mut self) -> Result<GeneId, ShahError> {
        let pos = self.items.file_size()?;
        if pos < ENTITY_META + T::N {
            return Ok(GeneId(1));
        }

        let sn = T::N * PAGE_SIZE as u64;
        let usabe = pos - (ENTITY_META + T::N);

        let (id, offset) = (usabe / sn, usabe % sn);
        if offset != 0 {
            log::warn!("{} new-stack-id bad offset: {offset}", self.ls);
        }

        Ok(GeneId(id * PAGE_SIZE as u64 + 1))
    }

    pub fn get(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        self.items.get(gene, entity)
    }

    pub fn count(&mut self) -> Result<EntityCount, ShahError> {
        self.items.count()
    }

    pub fn set(&mut self, entity: &mut T) -> Result<(), ShahError> {
        if !entity.is_alive() {
            return Err(NotFound::DeadSet)?;
        }

        let mut old_entity = T::default();
        self.items.get(entity.gene(), &mut old_entity)?;

        *entity.growth_mut() = old_entity.growth();
        *entity.pond_mut() = *old_entity.pond();
        self.items.set_unchecked(entity)?;

        Ok(())
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        self.items.del(gene, entity)?;

        let mut pond = Pond::default();
        let mut origin = Origin::default();

        self.index.get(entity.pond(), &mut pond)?;
        if pond.alive > 0 {
            pond.alive -= 1;
        }

        self.origins.get(&pond.origin, &mut origin)?;
        if origin.items > 0 {
            origin.items -= 1;
        }

        if pond.alive == 0 {
            self.add_empty_pond(&mut origin, pond)?;
        } else {
            self.index.set(&mut pond)?;
        }

        self.origins.set(&mut origin)?;

        Ok(())
    }

    pub fn list(
        &mut self, page: GeneId, result: &mut [T; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        self.items.list(page, result)
    }

    pub fn pond_list(
        &mut self, pond: &mut Pond, result: &mut [T; PAGE_SIZE],
    ) -> Result<(), ShahError> {
        let pond_gene = pond.gene;
        self.index.get(&pond_gene, pond)?;
        self.items.read_buf_at(result, pond.stack)?;
        Ok(())
    }

    pub fn pond_free(&mut self, pond: &mut Pond) -> Result<(), ShahError> {
        let mut buf = [T::default(); PAGE_SIZE];
        self.items.read_buf_at(&mut buf, pond.stack)?;

        pond.empty = 0;
        for item in buf.iter_mut() {
            if item.is_alive() {
                item.set_alive(false);
            }
            if item.gene().iter < ITER_EXHAUSTION {
                pond.empty += 1;
            }
        }

        self.items.write_buf_at(&buf, pond.stack)?;

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
            if let Err(e) = self.index.get(&pond_gene, &mut pond) {
                if e.is_not_found() {
                    break;
                }
                return Err(e)?;
            }
            pond_gene = pond.next;
            self.pond_free(&mut pond)?;
        }

        self.origins.del(origene, &mut origin)?;

        Ok(())
    }
}
