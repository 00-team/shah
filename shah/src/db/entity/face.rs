use std::fmt::Debug;

use crate::models::{Binary, Gene, ShahSchema};

macro_rules! flag {
    ($name:ident, $set:ident) => {
        fn $name(&self) -> bool;
        fn $set(&mut self, $name: bool) -> &mut Self;
    };
}

pub trait Entity {
    fn gene(&self) -> &Gene;
    fn gene_mut(&mut self) -> &mut Gene;
    fn growth(&self) -> u64;
    fn growth_mut(&mut self) -> &mut u64;

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

macro_rules! id_iter {
    ($name:ident) => {
        impl $name {
            pub fn end(&mut self) {
                self.prog = self.total;
            }

            pub fn ended(&self) -> bool {
                self.prog == self.total
            }
        }

        impl Iterator for $name {
            type Item = GeneId;
            fn next(&mut self) -> Option<Self::Item> {
                if self.prog >= self.total {
                    return None;
                }

                if self.prog == 0 {
                    self.prog += 1;
                }

                let id = self.prog;
                self.prog += 1;
                Some(id)
            }
        }
    };
}
pub(crate) use id_iter;
