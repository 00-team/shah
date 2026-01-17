use crate::{
    ShahModel,
    models::{Gene, ShahSchema},
};

macro_rules! flag {
    ($name:ident, $set:ident) => {
        fn $name(&self) -> bool;
        fn $set(&mut self, $name: bool) -> &mut Self;
    };
}

pub trait Entity: ShahModel {
    fn gene(&self) -> &Gene;
    fn gene_mut(&mut self) -> &mut Gene;
    fn growth(&self) -> u64;
    fn growth_mut(&mut self) -> &mut u64;

    flag! {is_alive, set_alive}
    flag! {is_dep_edited, set_dep_edited}
    flag! {is_dep_private, set_dep_private}
}

pub trait EntityItem: Entity + ShahSchema {}
impl<T: Entity + ShahSchema> EntityItem for T {}
