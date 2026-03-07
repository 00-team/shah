use super::*;
use crate::config::ShahConfig;
use crate::db::entity::{EntityDb, EntityKochFrom};
use crate::models::DeadList;
use crate::models::Worker;
use crate::models::task_list::{Performed, Task, TaskList};
use crate::utils;
use crate::{OptNotFound, ShahError};

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
            free_list: DeadList::new(),
            path: path.to_string(),
            item: EntityDb::new(path, revision)?,
            pond: EntityDb::new(&format!("{path}/index"), pond_revision)?,
            origin: EntityDb::new(&format!("{path}/origin"), origin_revision)?,
            tasks: TaskList::new([
                Self::work_item,
                Self::work_pond,
                Self::work_origin,
                Self::work_find_free,
            ]),
            pond_prog: Default::default(),
            insert_sequentially: false,
            ls: format!("<PondDb {path}.{revision} />"),
        };

        db.item.set_dead_list_disabled(true);

        db.pond_prog.total = db.pond.live + 1;
        db.pond_prog.prog = GeneId(1);

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

    fn work_find_free(&mut self) -> Result<Performed, ShahError> {
        if self.pond_prog.ended() {
            return Ok(Performed(false));
        }

        let mut pond = Pn::default();
        let mut performed = false;
        for _ in 0..10 {
            let Some(id) = self.pond_prog.next() else { break };
            performed = true;

            if self.pond.read_at(&mut pond, id).onf()?.is_none() {
                self.pond_prog.end();
                log::warn!(
                    "{} work_fine_free read_at not found {id:?}",
                    self.ls
                );
                break;
            }

            if self.path.starts_with("topic") {
                log::info!("pond: {pond:#?}");
            }
            if pond.origin().is_some()
                || pond.next().is_some()
                || pond.past().is_some()
            {
                continue;
            }

            if pond.alive() == 0 && pond.empty() > 0 {
                self.free_list.push(*pond.gene());
            }
        }

        Ok(Performed(performed))
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
> Worker<4> for PondDb<Dk, Pn, Og, DkO, PnO, OgO, DkS, PnS, OgS>
{
    fn tasks(&mut self) -> &mut TaskList<4, Task<Self>> {
        &mut self.tasks
    }
}
