use super::entity::{
    Entity, EntityCount, EntityDb, EntityItem, EntityKochFrom, ENTITY_META,
};
use crate::config::ShahConfig;
use crate::models::task_list::{Performed, Task, TaskList};
use crate::models::{Binary, DeadList, Gene, GeneId};
use crate::ShahError;
use crate::{utils, OptNotFound, SystemError, BLOCK_SIZE, PAGE_SIZE};

use std::fmt::Debug;
use std::path::Path;

mod index;
mod init;
mod public;
mod util;

// NOTE's for sorted ponds.
// 1. we dont need to sort items in each "stack" and we can just leave that
//    for frontend. just have a min/max value in each "pond" and
//    if item > pond.max then move it to pond.past ...

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, Clone, crate::Entity)]
pub struct Origin {
    pub gene: Gene,
    pub head: Gene,
    pub tail: Gene,
    pub owner: Gene,
    pub ponds: u64,
    pub items: u64,
    entity_flags: u8,
    _pad: [u8; 7],
    growth: u64,
}

pub trait Duck {
    fn pond(&self) -> &Gene;
    fn pond_mut(&mut self) -> &mut Gene;
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, crate::Entity, Clone)]
pub struct Pond {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub origin: Gene,
    pub stack: GeneId,
    pub growth: u64,
    pub entity_flags: u8,
    // NOTE: is_free flags is set but never read
    #[flags(is_free)]
    pub flags: u8,
    pub alive: u8,
    /// not iter exhausted slots.
    /// in other words slots that did not used all of their gene.iter
    pub empty: u8,
    _pad: [u8; 4],
}

pub trait PondItem: EntityItem + Duck + Copy {}
impl<T: EntityItem + Duck + Copy> PondItem for T {}

#[derive(Debug)]
pub struct PondDb<T: PondItem + EntityKochFrom<O, S>, O: EntityItem = T, S = ()>
{
    pub index: EntityDb<Pond>,
    pub origins: EntityDb<Origin>,
    free_list: DeadList<Gene, BLOCK_SIZE>,
    ls: String,
    items: EntityDb<T, O, S>,
    tasks: TaskList<3, Task<Self>>,
}
