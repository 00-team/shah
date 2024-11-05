use super::entity::{Entity, EntityDb};
use crate::error::SystemError;
use crate::{Binary, Gene, PAGE_SIZE};
use std::fmt::Debug;

#[crate::model]
#[derive(Debug, Clone, Copy, crate::Entity)]
pub struct Origin {
    pub gene: Gene,
    pub owner: Gene,
    pub broods: u64,
    pub items: u64,
    pub first: Gene,
    pub last: Gene,
    #[entity_flags]
    pub entity_flags: u8,
    pub _pad: [u8; 7],
}

#[shah_macros::model]
#[derive(Debug, Clone, Copy)]
pub struct PondChild {
    brood: Gene,
    alive: u8,
    #[flags(free)]
    flags: u8,
    _pad: [u8; 6],
}

#[shah_macros::model]
#[derive(Debug, shah_macros::Entity, Clone)]
pub struct PondIndex {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub origin: Gene,
    #[entity_flags]
    pub entity_flags: u8,
    _pad: [u8; 7],
    pub children: [PondChild; 10],
}

#[shah_macros::model]
#[derive(Debug, shah_macros::Entity, Clone)]
pub struct Brood<T: Default + Copy> {
    pub gene: Gene,
    pub pond: Gene,
    #[entity_flags]
    pub entity_flags: u8,
    pub alive: u8,
    pub _pad: [u8; 6],
    pub children: [T; PAGE_SIZE],
}

#[derive(Debug)]
pub struct PondDb<T>
where
    T: Default + Entity + Debug + Clone + Copy + Binary,
{
    // pub live: u64,
    // pub free: u64,
    // pub free_list: [GeneId; BLOCK_SIZE],
    pub index: EntityDb<PondIndex>,
    pub items: EntityDb<Brood<T>>,
    pub origins: EntityDb<Origin>,
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
            index: EntityDb::<PondIndex>::new(&format!("{name}.pond.index"))?,
            items: EntityDb::<Brood<T>>::new(&format!("{name}.pond.brood"))?,
            origins: EntityDb::<Origin>::new(&format!("{name}.pond.origin"))?,
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

    pub fn add(
        &mut self, origin_gene: &Gene, item: &mut T,
    ) -> Result<(), SystemError> {
        let mut origin = Origin::default();
        self.origins.get(origin_gene, &mut origin)?;

        let mut pond_gene = origin.first.clone();
        let mut pond = PondIndex::default();
        let brood = loop {
            self.index.get(&pond_gene, &mut pond)?;
            if let Some(child) = pond
                .children
                .iter()
                .find(|c| c.brood.is_some() && c.alive < PAGE_SIZE as u8)
            {
                break Some(child.brood.clone());
            }
            if pond.next.is_none() {
                break None;
            }
            pond_gene = pond.next;
        };

        Ok(())
    }
}
