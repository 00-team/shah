use crate::db::entity::EntityKoch;

use super::*;

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
    pub fn set_koch(
        &mut self, koch: EntityKoch<Dk, DkO, DkS>,
    ) -> Result<(), ShahError> {
        self.items.set_koch(koch)
    }

    pub fn set_work_iter(&mut self, work_iter: usize) {
        self.items.set_work_iter(work_iter);
    }
}
