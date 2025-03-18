use crate::db::entity::EntityKoch;

use super::*;

impl<T: PondItem + EntityKochFrom<O, S>, O: EntityItem, S> PondDb<T, O, S> {
    pub fn set_koch(
        &mut self, koch: EntityKoch<T, O, S>,
    ) -> Result<(), ShahError> {
        self.items.set_koch(koch)
    }

    pub fn set_work_iter(&mut self, work_iter: usize) {
        self.items.set_work_iter(work_iter);
    }
}
