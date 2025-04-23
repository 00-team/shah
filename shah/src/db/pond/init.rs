use super::*;
use crate::models::Worker;

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
    pub fn new(path: &str, revision: u16) -> Result<Self, ShahError> {
        ShahConfig::get();
        let data_path = Path::new("data/").join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let mut db = Self {
            free_list: DeadList::<Gene, BLOCK_SIZE>::new(),
            index: EntityDb::<Pn, PnO, PnS>::new(&format!("{path}/index"), 0)?,
            origins: EntityDb::<Og, OgO, OgS>::new(
                &format!("{path}/origin"),
                0,
            )?,
            items: EntityDb::<Dk, DkO, DkS>::new(path, revision)?,
            tasks: TaskList::new([
                Self::work_index,
                Self::work_origins,
                Self::work_items,
            ]),
            ls: format!("<PondDb {path}.{revision} />"),
        };

        db.items.set_dead_list_disabled(true);

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
