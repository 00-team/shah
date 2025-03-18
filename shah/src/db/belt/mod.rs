use crate::db::entity::{
    Entity, EntityCount, EntityDb, EntityItem, EntityKochFrom,
};
use crate::models::{Gene, GeneId, Performed, Task, TaskList};
use crate::{
    utils, IsNotFound, OptNotFound, ShahError, SystemError, PAGE_SIZE,
};
use std::path::Path;

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, Clone, crate::Entity)]
pub struct Buckle {
    pub gene: Gene,
    pub head: Gene,
    pub tail: Gene,
    pub belts: u64,
    growth: u64,
    entity_flags: u8,
    _pad: [u8; 7],
}

pub trait Belt: EntityItem {
    fn next(&self) -> &Gene;
    fn next_mut(&mut self) -> &mut Gene;
    fn past(&self) -> &Gene;
    fn past_mut(&mut self) -> &mut Gene;
    fn buckle(&self) -> &Gene;
    fn buckle_mut(&mut self) -> &mut Gene;
}

#[derive(Debug)]
pub struct BeltDb<B: Belt + EntityKochFrom<OB, BS>, OB: Belt = B, BS = ()> {
    buckle: EntityDb<Buckle>,
    belt: EntityDb<B, OB, BS>,
    ls: String,
    tasks: TaskList<2, Task<Self>>,
}

impl<B: Belt + EntityKochFrom<OB, BS>, OB: Belt, BS> BeltDb<B, OB, BS> {
    pub fn new(path: &str, revision: u16) -> Result<Self, ShahError> {
        let data_path = Path::new("data/").join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let db = Self {
            belt: EntityDb::<B, OB, BS>::new(
                &format!("{path}/belt"),
                revision,
            )?,
            buckle: EntityDb::<Buckle>::new(&format!("{path}/buckle"), 0)?,
            tasks: TaskList::new([Self::work_belt, Self::work_buckle]),
            ls: format!("<BeltDb {path} />"),
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
}

impl<B: Belt + EntityKochFrom<OB, BS>, OB: Belt, BS> BeltDb<B, OB, BS> {
    pub fn buckle_add(&mut self, buckle: &mut Buckle) -> Result<(), ShahError> {
        buckle.set_alive(true);
        buckle.belts = 0;
        buckle.growth = 0;
        buckle.head.clear();
        buckle.tail.clear();

        self.buckle.add(buckle)
    }

    pub fn buckle_get(
        &mut self, gene: &Gene, buckle: &mut Buckle,
    ) -> Result<(), ShahError> {
        self.buckle.get(gene, buckle)
    }

    pub fn buckle_count(&mut self) -> Result<EntityCount, ShahError> {
        self.buckle.count()
    }

    pub fn buckle_list(
        &mut self, page: GeneId, result: &mut [Buckle; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        self.buckle.list(page, result)
    }

    /// this will cascade all the belts under this buckle
    pub fn buckle_del(&mut self, gene: &Gene) -> Result<(), ShahError> {
        let mut buckle = Buckle::default();
        self.buckle.get(gene, &mut buckle)?;

        let mut belt_gene = buckle.head;
        let mut belt = B::default();
        loop {
            if let Err(e) = self.belt.del(&belt_gene, &mut belt) {
                if e.is_not_found() {
                    break;
                }
                return Err(e)?;
            }
            belt_gene = *belt.next();
        }

        self.buckle.del_unchecked(&mut buckle)
    }
}

impl<B: Belt + EntityKochFrom<OB, BS>, OB: Belt, BS> BeltDb<B, OB, BS> {
    pub fn belt_add(
        &mut self, buckle_gene: &Gene, belt: &mut B,
    ) -> Result<(), ShahError> {
        belt.set_alive(true);

        let mut buckle = Buckle::default();
        self.buckle.get(buckle_gene, &mut buckle)?;

        *belt.buckle_mut() = buckle.gene;
        *belt.growth_mut() = 0;
        *belt.past_mut() = buckle.tail;
        belt.next_mut().clear();

        self.belt.add(belt)?;

        let old_tail_gene = buckle.tail;
        buckle.tail = *belt.gene();
        buckle.belts += 1;

        if self.belt.get(&old_tail_gene, belt).onf()?.is_some() {
            *belt.next_mut() = buckle.tail;
            self.belt.set(belt)?;
        }

        self.buckle.set(&mut buckle)
    }

    pub fn belt_get(
        &mut self, gene: &Gene, belt: &mut B,
    ) -> Result<(), ShahError> {
        self.belt.get(gene, belt)
    }

    pub fn belt_count(&mut self) -> Result<EntityCount, ShahError> {
        self.belt.count()
    }

    pub fn belt_set(&mut self, belt: &mut B) -> Result<(), ShahError> {
        if !belt.is_alive() {
            log::error!("{} DeadSet: using set to delete", self.ls);
            return Err(SystemError::DeadSet)?;
        }

        let mut old_belt = B::default();
        self.belt.get(belt.gene(), &mut old_belt)?;

        *belt.growth_mut() = old_belt.growth();
        *belt.next_mut() = *old_belt.next();
        *belt.past_mut() = *old_belt.past();
        *belt.buckle_mut() = *old_belt.buckle();

        self.belt.set_unchecked(belt)
    }

    pub fn belt_del(
        &mut self, gene: &Gene, belt: &mut B,
    ) -> Result<(), ShahError> {
        self.belt.get(gene, belt)?;

        let mut buckle = Buckle::default();
        self.buckle.get(belt.buckle(), &mut buckle)?;

        if buckle.belts > 0 {
            buckle.belts -= 1;
        }

        if buckle.head == *belt.gene() {
            buckle.head = *belt.next();
        }

        if buckle.tail == *belt.gene() {
            buckle.tail = *belt.past();
        }

        let mut sibling = B::default();

        if let Err(e) = self.belt.get(belt.past(), &mut sibling) {
            e.not_found_ok()?;
        } else {
            *sibling.next_mut() = *belt.next();
            self.belt.set_unchecked(&mut sibling)?;
        }

        if let Err(e) = self.belt.get(belt.next(), &mut sibling) {
            e.not_found_ok()?;
        } else {
            *sibling.past_mut() = *belt.past();
            self.belt.set_unchecked(&mut sibling)?;
        }

        self.belt.del_unchecked(belt)?;

        self.buckle.set_unchecked(&mut buckle)
    }

    pub fn belt_list(
        &mut self, page: GeneId, result: &mut [B; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        self.belt.list(page, result)
    }
}
