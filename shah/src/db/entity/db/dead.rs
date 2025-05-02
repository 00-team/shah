use super::*;

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub(super) fn take_dead_id(&mut self) -> GeneId {
        if self.dead_list.disabled() {
            return GeneId(0);
        }
        self.dead_list.pop(|_| true).unwrap_or_default()
    }

    pub(super) fn dead_add(&mut self, gene: &Gene) {
        if gene.id == 0 {
            return;
        }

        self.live -= 1;

        if gene.exhausted() || self.dead_list.disabled() {
            return;
        }

        self.dead_list.push(gene.id);
    }
}
