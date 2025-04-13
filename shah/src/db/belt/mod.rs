use crate::db::entity::{
    Entity, EntityCount, EntityDb, EntityItem, EntityKochFrom,
};
use crate::models::{Gene, GeneId, Performed, Task, TaskList, Worker};
use crate::{
    utils, IsNotFound, OptNotFound, ShahError, SystemError, PAGE_SIZE,
};
use std::path::Path;

mod belt_api;
mod buckle;

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, Clone, Copy, crate::Entity)]
pub struct Buckle {
    pub gene: Gene,
    pub head: Gene,
    pub tail: Gene,
    pub belts: u64,
    pub owner: Gene,
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

    // pub fn work(&mut self) -> Result<Performed, ShahError> {
    //     self.tasks.start();
    //     while let Some(task) = self.tasks.next() {
    //         if task(self)?.0 {
    //             return Ok(Performed(true));
    //         }
    //     }
    //     Ok(Performed(false))
    // }
}

impl<B: Belt + EntityKochFrom<OB, BS>, OB: Belt, BS> Worker<2>
    for BeltDb<B, OB, BS>
{
    fn tasks(&mut self) -> &mut TaskList<2, Task<Self>> {
        &mut self.tasks
    }
}
