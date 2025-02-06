use std::fmt::Debug;

use crate::models::{Binary, Gene, ShahSchema};
use crate::ShahError;

macro_rules! flag {
    ($name:ident, $set:ident) => {
        fn $name(&self) -> bool;
        fn $set(&mut self, $name: bool) -> &mut Self;
    };
}

pub trait Entity {
    fn gene(&self) -> &Gene;
    fn gene_mut(&mut self) -> &mut Gene;

    flag! {is_alive, set_alive}
    flag! {is_edited, set_edited}
    flag! {is_private, set_private}
}

pub trait EntityItem:
    Default + Entity + Debug + Clone + Binary + ShahSchema
{
}
impl<T: Default + Entity + Debug + Clone + Binary + ShahSchema> EntityItem
    for T
{
}

pub trait EntityMigrateFrom<Old: EntityItem, State = ()>: Sized {
    fn entity_migrate_from(old: Old, state: State) -> Result<Self, ShahError>;
}
impl<Old: EntityItem, State> EntityMigrateFrom<Old, State> for Old {
    fn entity_migrate_from(old: Old, _: State) -> Result<Self, ShahError> {
        Ok(old)
    }
}
