use crate::db::entity::{Entity, EntityDb};
use crate::models::{Gene, Performed, Task, TaskList};
use crate::{utils, ShahError};
use std::path::Path;

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, Clone, crate::Entity)]
struct Buckle {
    pub gene: Gene,
    pub belts: u64,
    pub first: Gene,
    pub last: Gene,
    pub length: u64,
    growth: u64,
    pub entity_flags: u8,
    pub _pad: [u8; 7],
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, crate::Entity, Clone)]
pub struct Belt<const N: usize> {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub buckle: Gene,
    pub length: u64,
    pub growth: u64,
    pub entity_flags: u8,
    #[flags(free)]
    pub flags: u8,
    #[str]
    pub data: [u8; N],
}

pub struct BeltDb<const N: usize> {
    buckle: EntityDb<Buckle>,
    belt: EntityDb<Belt<N>>,
    ls: String,
    tasks: TaskList<2, Task<Self>>,
}

impl<const N: usize> BeltDb<N> {
    pub fn new(path: &str) -> Result<Self, ShahError> {
        let data_path = Path::new("data/").join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let db = Self {
            belt: EntityDb::<Belt<N>>::new(&format!("{path}/belt"), 0)?,
            buckle: EntityDb::<Buckle>::new(&format!("{path}/buckle"), 0)?,
            tasks: TaskList::new([Self::work_belt, Self::work_buckle]),
            ls: format!("<BeltDb {name} />"),
        };

        Ok(db)
    }

    fn work_belt(&mut self) -> Result<Performed, ShahError> {
        self.belt.work()
    }

    fn work_buckle(&mut self) -> Result<Performed, ShahError> {
        self.buckle.work()
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

    // pub fn add(
    //     &mut self, buckle_gene: &Gene, belt: &mut Belt<N>,
    // ) -> Result<(), ShahError> {
    //     belt.set_alive(true);
    //
    //     let mut buckle = Buckle::default();
    //     self.buckle.get(buckle_gene, &mut buckle)?;
    //     buckle;
    //
    //     let mut pond = self.half_empty_pond(&mut origin)?;
    //     pond.alive += 1;
    //
    //     let mut buf = [T::default(); PAGE_SIZE];
    //     *item.pond_mut() = pond.gene;
    //     let ig = item.gene_mut();
    //     ig.server = 69;
    //     crate::utils::getrandom(&mut ig.pepper);
    //
    //     let stack = if pond.stack == 0 {
    //         let stack = self.new_stack_id()?;
    //         ig.id = stack;
    //         ig.iter = 0;
    //         buf[0] = *item;
    //
    //         pond.stack = stack;
    //         pond.empty = PAGE_SIZE as u8 - 1;
    //         stack
    //     } else {
    //         self.items.read_buf_at(&mut buf, pond.stack)?;
    //
    //         let mut found_empty_slot = false;
    //         for (x, slot) in buf.iter_mut().enumerate() {
    //             let sg = slot.gene();
    //             if !slot.is_alive() && sg.iter < ITER_EXHAUSTION {
    //                 let ig = item.gene_mut();
    //                 ig.id = pond.stack + x as u64;
    //                 ig.iter = if sg.id != 0 { sg.iter + 1 } else { 0 };
    //                 *slot = *item;
    //                 found_empty_slot = true;
    //                 if pond.empty > 0 {
    //                     pond.empty -= 1;
    //                 }
    //                 break;
    //             }
    //         }
    //         if !found_empty_slot {
    //             log::error!("could not found an empty slot for item");
    //         }
    //
    //         pond.stack
    //     };
    //
    //     self.items.write_buf_at(&buf, stack)?;
    //     self.index.set(&mut pond)?;
    //     self.origins.set(&mut origin)?;
    //
    //     Ok(())
    // }
    //
    // pub fn new_stack_id(&mut self) -> Result<GeneId, ShahError> {
    //     let pos = self.items.file_size()?;
    //     if pos < ENTITY_META + T::N {
    //         return Ok(GeneId(1));
    //     }
    //
    //     let sn = T::N * PAGE_SIZE as u64;
    //     let usabe = pos - (ENTITY_META + T::N);
    //
    //     let (id, offset) = (usabe / sn, usabe % sn);
    //     if offset != 0 {
    //         log::warn!("{} new-stack-id bad offset: {offset}", self.ls);
    //     }
    //
    //     Ok(GeneId(id * PAGE_SIZE as u64 + 1))
    // }
    //
    // pub fn get(
    //     &mut self, gene: &Gene, entity: &mut T,
    // ) -> Result<(), ShahError> {
    //     self.items.get(gene, entity)
    // }
    //
    // pub fn count(&mut self) -> Result<EntityCount, ShahError> {
    //     self.items.count()
    // }
    //
    // pub fn set(&mut self, entity: &mut T) -> Result<(), ShahError> {
    //     if !entity.is_alive() {
    //         return Err(NotFound::DeadSet)?;
    //     }
    //
    //     let mut old_entity = T::default();
    //     self.items.get(entity.gene(), &mut old_entity)?;
    //
    //     *entity.growth_mut() = old_entity.growth();
    //     *entity.pond_mut() = *old_entity.pond();
    //     self.items.set_unchecked(entity)?;
    //
    //     Ok(())
    // }
    //
    // pub fn del(
    //     &mut self, gene: &Gene, entity: &mut T,
    // ) -> Result<(), ShahError> {
    //     self.items.del(gene, entity)?;
    //
    //     let mut pond = Pond::default();
    //     let mut origin = Buckle::default();
    //
    //     self.index.get(entity.pond(), &mut pond)?;
    //     if pond.alive > 0 {
    //         pond.alive -= 1;
    //     }
    //
    //     self.origins.get(&pond.origin, &mut origin)?;
    //     if origin.items > 0 {
    //         origin.items -= 1;
    //     }
    //
    //     if pond.alive == 0 {
    //         self.add_empty_pond(&mut origin, pond)?;
    //     } else {
    //         self.index.set(&mut pond)?;
    //     }
    //
    //     self.origins.set(&mut origin)?;
    //
    //     Ok(())
    // }
    //
    // pub fn list(
    //     &mut self, page: GeneId, result: &mut [T; PAGE_SIZE],
    // ) -> Result<usize, ShahError> {
    //     self.items.list(page, result)
    // }
    //
    // pub fn pond_list(
    //     &mut self, pond: &mut Pond, result: &mut [T; PAGE_SIZE],
    // ) -> Result<(), ShahError> {
    //     let pond_gene = pond.gene;
    //     self.index.get(&pond_gene, pond)?;
    //     self.items.read_buf_at(result, pond.stack)?;
    //     Ok(())
    // }
    //
    // pub fn pond_free(&mut self, pond: &mut Pond) -> Result<(), ShahError> {
    //     let mut buf = [T::default(); PAGE_SIZE];
    //     self.items.read_buf_at(&mut buf, pond.stack)?;
    //
    //     pond.empty = 0;
    //     for item in buf.iter_mut() {
    //         if item.is_alive() {
    //             item.set_alive(false);
    //         }
    //         if item.gene().iter < ITER_EXHAUSTION {
    //             pond.empty += 1;
    //         }
    //     }
    //
    //     self.items.write_buf_at(&buf, pond.stack)?;
    //
    //     pond.set_free(true);
    //     pond.alive = 0;
    //
    //     self.index.set(pond)?;
    //     self.free_list.push(pond.gene);
    //
    //     Ok(())
    // }
    //
    // pub fn cascade(&mut self, origene: &Gene) -> Result<(), ShahError> {
    //     let mut origin = Buckle::default();
    //     self.origins.get(origene, &mut origin)?;
    //
    //     let mut pond_gene = origin.first;
    //     let mut pond = Pond::default();
    //     loop {
    //         if let Err(e) = self.index.get(&pond_gene, &mut pond) {
    //             if e.is_not_found() {
    //                 break;
    //             }
    //             return Err(e)?;
    //         }
    //         pond_gene = pond.next;
    //         self.pond_free(&mut pond)?;
    //     }
    //
    //     self.origins.del(origene, &mut origin)?;
    //
    //     Ok(())
    // }
}
