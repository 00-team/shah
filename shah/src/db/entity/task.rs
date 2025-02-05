use std::fmt::Debug;
use std::io::ErrorKind;
use std::os::unix::fs::FileExt;

use crate::db::entity::META_OFFSET;
use crate::error::ShahError;
use crate::state::Task;

use super::face::{EntityItem, EntityMigrateFrom};
use super::EntityDb;

#[derive(Debug)]
pub struct EntitySetupTask<
    T: EntityItem + EntityMigrateFrom<S, O>,
    O: EntityItem,
    S: Debug + 'static,
> {
    pub total: u64,
    pub progress: u64,
    pub db: &'static mut EntityDb<T, O, S>,
}

impl<'edb, 'state, T: EntityItem + EntityMigrateFrom<S, O>, O: EntityItem, S: Debug> Task
    for EntitySetupTask<'edb, 'state, T, O, S>
{
    fn work(&mut self) -> Result<bool, ShahError> {
        log::info!(
            "entity setup task | total: {} | progress: {}",
            self.total,
            self.progress
        );
        if self.total <= self.progress {
            return Ok(true);
        }

        if self.progress == 0 {
            self.progress += 1;
        }

        let a = (self.total - self.progress).min(10);
        let mut entity = T::default();
        for i in 0..a {
            let pos = META_OFFSET + (self.progress + i) * T::N;

            log::info!("read at: {pos}");
            match self.db.file.read_exact_at(entity.as_binary_mut(), pos) {
                Ok(_) => {}
                Err(e) => match e.kind() {
                    ErrorKind::UnexpectedEof => return Ok(true),
                    _ => return Err(e)?,
                },
            }

            if !entity.is_alive() {
                let gene = entity.gene();
                log::debug!("dead entity: {entity:?}");
                self.db.add_dead(gene);
            }

            // f(&mut self, &entity);
        }

        Ok(self.progress >= self.total)
    }
}

#[derive(Debug)]
pub struct EntityMigrateTask {}
impl Task for EntityMigrateTask {
    fn work(&mut self) -> Result<bool, ShahError> {
        log::info!("entity mig task done");
        Ok(true)
    }
}
