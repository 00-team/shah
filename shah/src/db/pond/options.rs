use super::*;
use crate::ShahError;
use crate::db::entity::EntityKoch;
use crate::db::entity::EntityKochFrom;

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
        self.item.set_koch(koch)
    }

    pub fn set_pond_koch(
        &mut self, koch: EntityKoch<Pn, PnO, PnS>,
    ) -> Result<(), ShahError> {
        self.pond.set_koch(koch)
    }

    pub fn set_origin_koch(
        &mut self, koch: EntityKoch<Og, OgO, OgS>,
    ) -> Result<(), ShahError> {
        self.origin.set_koch(koch)
    }

    pub fn set_work_iter(&mut self, work_iter: usize) {
        self.item.set_work_iter(work_iter);
    }
}
