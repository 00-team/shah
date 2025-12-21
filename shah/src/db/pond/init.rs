use super::*;
use crate::ShahError;
use crate::config::ShahConfig;
use crate::db::entity::{EntityDb, EntityKochFrom};
use crate::models::Worker;
use crate::models::task_list::{Performed, Task, TaskList};
use crate::models::{DeadList, Gene};
use crate::{BLOCK_SIZE, utils};

impl<
    Dk: Duck + EntityKochFrom<DkO, DkS>,
    Pn: Pond + EntityKochFrom<PnO, PnS>,
    Og: Origin + EntityKochFrom<OgO, OgS>,
    DkO: Duck,
    PnO: Pond,
    OgO: Origin,
    DkS,
    PnS,
    OgS,
> PondDb<Dk, Pn, Og, DkO, PnO, OgO, DkS, PnS, OgS>
{
    pub fn new(
        path: &str, revision: u16, pond_revision: u16, origin_revision: u16,
    ) -> Result<Self, ShahError> {
        let conf = ShahConfig::get();
        let data_path = conf.data_dir.join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let mut db = Self {
            free_list: DeadList::<Gene, BLOCK_SIZE>::new(),
            item: EntityDb::<Dk, DkO, DkS>::new(path, revision)?,
            pond: EntityDb::new(&format!("{path}/index"), pond_revision)?,
            origin: EntityDb::new(&format!("{path}/origin"), origin_revision)?,
            tasks: TaskList::new([
                Self::work_item,
                Self::work_pond,
                Self::work_origin,
            ]),
            ls: format!("<PondDb {path}.{revision} />"),
        };

        db.item.set_dead_list_disabled(true);

        Ok(db)
    }

    fn work_item(&mut self) -> Result<Performed, ShahError> {
        self.item.work()
    }

    fn work_pond(&mut self) -> Result<Performed, ShahError> {
        self.pond.work()
    }

    fn work_origin(&mut self) -> Result<Performed, ShahError> {
        self.origin.work()
    }
}

impl<
    Dk: Duck + EntityKochFrom<DkO, DkS>,
    Pn: Pond + EntityKochFrom<PnO, PnS>,
    Og: Origin + EntityKochFrom<OgO, OgS>,
    DkO: Duck,
    PnO: Pond,
    OgO: Origin,
    DkS,
    PnS,
    OgS,
> Worker<3> for PondDb<Dk, Pn, Og, DkO, PnO, OgO, DkS, PnS, OgS>
{
    fn tasks(&mut self) -> &mut TaskList<3, Task<Self>> {
        &mut self.tasks
    }
}
