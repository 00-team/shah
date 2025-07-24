use crate::models::GeneId;

mod db;
mod face;
mod koch;
mod meta;

pub use db::*;
pub use face::*;
pub use koch::*;
pub use meta::*;

#[derive(Debug)]
pub struct EntityCount {
    pub alive: GeneId,
    pub total: GeneId,
    pub size: u64
}
