use super::*;

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub fn set_koch(
        &mut self, koch: EntityKoch<T, O, S>,
    ) -> Result<(), ShahError> {
        self.koch_prog.total = koch.total;

        if !self.koch_prog.ended() {
            self.setup_prog.total = self.koch_prog.prog;
            self.setup_prog.prog = GeneId(1);
        }

        if self.live < koch.total {
            self.live = koch.total;
            utils::falloc(&self.file, ENTITY_META, (koch.total * T::N).0)?;
        }

        self.koch = Some(koch);

        Ok(())
    }

    pub fn set_inspector(&mut self, inspector: EntityInspector<T, Is>) {
        self.inspector = Some(inspector);
    }

    pub fn set_dead_list_disabled(&mut self, disabled: bool) {
        self.dead_list.disable(disabled)
    }

    pub fn set_work_iter(&mut self, work_iter: usize) {
        self.work_iter = work_iter.clamp(5, 100);
    }
}
