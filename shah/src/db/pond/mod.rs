use super::entity::{EntityDb, EntityItem, EntityKochFrom};
use crate::BLOCK_SIZE;
use crate::db::entity::EntityFlags;
use crate::models::task_list::{Task, TaskList};
use crate::models::{DeadList, Gene, GeneId};

use std::fmt::Debug;

mod api_item;
mod api_origin;
mod api_pond;

mod init;
mod options;
mod util;

// NOTE's for sorted ponds.
// 1. we dont need to sort items in each "stack" and we can just leave that
//    for frontend. just have a min/max value in each "pond" and
//    if item > pond.max then move it to pond.past ...

pub trait Origin: EntityItem {
    fn head(&self) -> &Gene;
    fn head_mut(&mut self) -> &mut Gene;
    fn tail(&self) -> &Gene;
    fn tail_mut(&mut self) -> &mut Gene;

    fn pond_count(&self) -> u64;
    fn pond_count_mut(&mut self) -> &mut u64;

    fn item_count(&self) -> u64;
    fn item_count_mut(&mut self) -> &mut u64;
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, crate::Entity, crate::Origin)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, utoipa::ToSchema))]
pub struct ShahOrigin {
    pub gene: Gene,
    pub head: Gene,
    pub tail: Gene,
    pub pond_count: u64,
    pub item_count: u64,
    entity_flags: EntityFlags,
    #[cfg_attr(feature = "serde", serde(skip))]
    _pad: [u8; 7],
    growth: u64,
}

pub trait Pond: EntityItem {
    fn next(&self) -> &Gene;
    fn next_mut(&mut self) -> &mut Gene;
    fn past(&self) -> &Gene;
    fn past_mut(&mut self) -> &mut Gene;
    fn origin(&self) -> &Gene;
    fn origin_mut(&mut self) -> &mut Gene;

    fn stack(&self) -> GeneId;
    fn stack_mut(&mut self) -> &mut GeneId;

    fn alive(&self) -> u8;
    fn alive_mut(&mut self) -> &mut u8;
    fn empty(&self) -> u8;
    fn empty_mut(&mut self) -> &mut u8;
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, crate::Entity, crate::Pond)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, utoipa::ToSchema))]
pub struct ShahPond {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub origin: Gene,
    pub stack: GeneId,
    pub growth: u64,
    pub entity_flags: EntityFlags,
    // NOTE: is_free flags is set but never read
    // #[flags(is_free)]
    // pub flags: u8,
    pub alive: u8,
    /// not iter exhausted slots.
    /// in other words slots that did not used all of their gene.iter
    pub empty: u8,
    #[cfg_attr(feature = "serde", serde(skip))]
    _pad: [u8; 5],
}

pub trait Duck: EntityItem {
    fn pond(&self) -> &Gene;
    fn pond_mut(&mut self) -> &mut Gene;
}

#[derive(Debug)]
pub struct PondDb<
    Dk: Duck + EntityKochFrom<DkO, DkS>,
    Pn: Pond + EntityKochFrom<PnO, PnS> = ShahPond,
    Og: Origin + EntityKochFrom<OgO, OgS> = ShahOrigin,
    // old
    DkO: Duck = Dk,
    PnO: Pond = Pn,
    OgO: Origin = Og,
    // state
    DkS = (),
    PnS = (),
    OgS = (),
> {
    item: EntityDb<Dk, DkO, DkS>,
    pond: EntityDb<Pn, PnO, PnS>,
    origin: EntityDb<Og, OgO, OgS>,
    free_list: DeadList<Gene, BLOCK_SIZE>,
    ls: String,
    tasks: TaskList<3, Task<Self>>,
}
