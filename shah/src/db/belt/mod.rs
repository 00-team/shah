use crate::config::ShahConfig;
use crate::db::entity::{
    EntityCount, EntityDb, EntityFlags, EntityItem, EntityKochFrom,
};
use crate::models::{Gene, GeneId, Performed, Task, TaskList, Worker};
use crate::{
    IsNotFound, OptNotFound, PAGE_SIZE, ShahError, SystemError, utils,
};

mod belt_api;
mod buckle;
pub mod cloth;
mod options;

pub trait Buckle: EntityItem {
    fn head(&self) -> &Gene;
    fn head_mut(&mut self) -> &mut Gene;
    fn tail(&self) -> &Gene;
    fn tail_mut(&mut self) -> &mut Gene;
    fn belt_count(&self) -> u64;
    fn belt_count_mut(&mut self) -> &mut u64;
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, crate::Entity, crate::Buckle)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, utoipa::ToSchema))]
pub struct ShahBuckle {
    pub gene: Gene,
    pub head: Gene,
    pub tail: Gene,
    pub belt_count: u64,
    growth: u64,
    entity_flags: EntityFlags,
    #[cfg_attr(feature = "serde", serde(skip))]
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
pub struct BeltDb<
    Bt: Belt + EntityKochFrom<BtO, BtS>,
    Bk: Buckle + EntityKochFrom<BkO, BkS> = ShahBuckle,
    BtO: Belt = Bt,
    BkO: Buckle = Bk,
    BtS = (),
    BkS = (),
> {
    buckle: EntityDb<Bk, BkO, BkS>,
    belt: EntityDb<Bt, BtO, BtS>,
    ls: String,
    tasks: TaskList<2, Task<Self>>,
}

impl<
    Bt: Belt + EntityKochFrom<BtO, BtS>,
    Bk: Buckle + EntityKochFrom<BkO, BkS>,
    BtO: Belt,
    BkO: Buckle,
    BtS,
    BkS,
> BeltDb<Bt, Bk, BtO, BkO, BtS, BkS>
{
    pub fn new(
        path: &str, revision: u16, buckle_revision: u16,
    ) -> Result<Self, ShahError> {
        let conf = ShahConfig::get();
        let data_path = conf.data_dir.join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let db = Self {
            belt: EntityDb::new(&format!("{path}/belt"), revision)?,
            buckle: EntityDb::new(&format!("{path}/buckle"), buckle_revision)?,
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

impl<
    Bt: Belt + EntityKochFrom<BtO, BtS>,
    Bk: Buckle + EntityKochFrom<BkO, BkS>,
    BtO: Belt,
    BkO: Buckle,
    BtS,
    BkS,
> Worker<2> for BeltDb<Bt, Bk, BtO, BkO, BtS, BkS>
{
    fn tasks(&mut self) -> &mut TaskList<2, Task<Self>> {
        &mut self.tasks
    }
}
