use crate::models::Worker;

use super::*;

impl<T: PondItem + EntityKochFrom<O, S>, O: EntityItem, S> PondDb<T, O, S> {
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
            index: EntityDb::<Pond>::new(&format!("{path}/index"), 0)?,
            origins: EntityDb::<Origin>::new(&format!("{path}/origin"), 0)?,
            items: EntityDb::<T, O, S>::new(path, revision)?,
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

impl<T: PondItem + EntityKochFrom<O, S>, O: EntityItem, S> Worker<3>
    for PondDb<T, O, S>
{
    fn tasks(&mut self) -> &mut TaskList<3, Task<Self>> {
        &mut self.tasks
    }
}
