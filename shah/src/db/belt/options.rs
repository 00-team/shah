use crate::db::entity::EntityKoch;

use super::*;

impl<
    Bt: Belt + EntityKochFrom<BtO, BtS>,
    Bk: Buckle + EntityKochFrom<BkO, BkS>,
    BtO: Belt,
    BkO: Buckle,
    BtS,
    BkS,
> BeltDb<Bt, Bk, BtO, BkO, BtS, BkS>
{
    pub fn set_koch(
        &mut self, koch: Option<EntityKoch<Bt, BtO, BtS>>,
    ) -> Result<(), ShahError> {
        self.belt.set_koch(koch)
    }

    pub fn set_buckle_koch(
        &mut self, koch: Option<EntityKoch<Bk, BkO, BkS>>,
    ) -> Result<(), ShahError> {
        self.buckle.set_koch(koch)
    }
}
