use crate::db::entity::EntityKoch;

use super::*;

impl<
    Dk: Duck + EntityKochFrom<DkO, DkS>,
    DkO: Duck,
    DkS,
    Pn: Pond + EntityKochFrom<PnO, PnS>,
    PnO: Pond,
    PnS,
    Og: Origin + EntityKochFrom<OgO, OgS>,
    OgO: Origin,
    OgS,
> PondDb<Dk, DkO, DkS, Pn, PnO, PnS, Og, OgO, OgS>
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
