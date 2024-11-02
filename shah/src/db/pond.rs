use super::entity::{Entity, EntityDb};
use crate::error::SystemError;
use crate::{Binary, Gene, GeneId};
use std::fmt::Debug;

#[shah_macros::model]
#[derive(Debug, Clone, Copy)]
pub struct PondChild {
    id: GeneId,
    alive: u8,
    _pad: [u8; 7],
}

#[shah_macros::model]
#[derive(Debug, shah_macros::Entity, Clone)]
pub struct PondIndex {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub parent: Gene,
    #[entity_flags]
    pub entity_flags: u8,
    _pad: [u8; 7],
    pub children: [PondChild; 10],
}

#[shah_macros::model]
#[derive(Debug, shah_macros::Entity, Clone)]
pub struct Brood<T: Default + Copy> {
    pub gene: Gene,
    pub parent: Gene,
    #[entity_flags]
    pub entity_flags: u8,
    pub alive: u8,
    pub _pad: [u8; 6],
    pub children: [T; 32],
}

#[derive(Debug)]
pub struct PondDb<T>
where
    T: Default + Entity + Debug + Clone + Copy + Binary,
{
    // pub live: u64,
    // pub dead: u64,
    // pub dead_list: [GeneId; BLOCK_SIZE],
    pub index: EntityDb<PondIndex>,
    pub items: EntityDb<Brood<T>>,
}

impl<T> PondDb<T>
where
    T: Entity + Debug + Clone + Copy + Default + Binary,
{
    pub fn new(name: &str) -> Result<Self, SystemError> {
        let db = Self {
            // live: 0,
            // dead: 0,
            // dead_list: [0; BLOCK_SIZE],
            index: EntityDb::<PondIndex>::new(&format!(
                "{name}.pond.index.bin"
            ))?,
            items: EntityDb::<Brood<T>>::new(&format!(
                "{name}.pond.brood.bin"
            ))?,
        };

        Ok(db)
    }

    pub fn setup(mut self) -> Result<Self, SystemError> {
        self.index = self.index.setup()?;
        self.items = self.items.setup()?;

        Ok(self)
    }

    pub fn get(
        &mut self, gene: &Gene, brood: &mut Brood<T>,
    ) -> Result<(), SystemError> {
        Ok(self.items.get(gene, brood)?)
    }
}
