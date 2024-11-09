use super::entity::{Entity, EntityDb};
use crate::error::SystemError;
use crate::{
    Binary, DeadList, Gene, GeneId, BLOCK_SIZE, ITER_EXHAUSTION, PAGE_SIZE,
};
use std::fmt::Debug;
use std::fs::File;
use std::marker::PhantomData;
use std::os::unix::fs::FileExt;

// NOTE's for sorted ponds.
// 1. we dont need to sort items in each "stack" and we can just leave that
//    for frontend. just have a min/max value in each "pond" and
//    if item > pond.max then move it to pond.past ...

#[crate::model]
#[derive(Debug, Clone, Copy, crate::Entity)]
pub struct Origin {
    pub gene: Gene,
    pub owner: Gene,
    pub ponds: u64,
    pub items: u64,
    pub first: Gene,
    pub last: Gene,
    #[entity_flags]
    pub entity_flags: u8,
    pub _pad: [u8; 7],
}

pub trait Duck {
    fn pond(&self) -> &Gene;
    fn pond_mut(&mut self) -> &mut Gene;
}

#[crate::model]
#[derive(Debug, crate::Entity, Clone)]
pub struct Pond {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub origin: Gene,
    pub stack: GeneId,
    #[entity_flags]
    pub entity_flags: u8,
    #[flags(free)]
    pub flags: u8,
    pub alive: u8,
    _pad: [u8; 5],
}

#[derive(Debug)]
pub struct PondDb<T: Duck> {
    pub file: File,
    pub live: u64,
    pub free_list: DeadList<Gene, BLOCK_SIZE>,
    pub index: EntityDb<Pond>,
    pub origins: EntityDb<Origin>,
    _e: PhantomData<T>,
}

impl<T: Default + Entity + Debug + Clone + Copy + Binary + Duck> PondDb<T> {
    pub fn new(name: &str) -> Result<Self, SystemError> {
        std::fs::create_dir_all("data/")?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.pond.items.bin"))?;

        let db = Self {
            file,
            live: 0,
            free_list: DeadList::<Gene, BLOCK_SIZE>::new(),
            index: EntityDb::<Pond>::new(&format!("{name}.pond.index"))?,
            // items: EntityDb::<Brood<T>>::new(&format!("{name}.pond.brood"))?,
            origins: EntityDb::<Origin>::new(&format!("{name}.pond.origin"))?,
            _e: PhantomData,
        };

        Ok(db)
    }

    pub fn setup(mut self) -> Result<Self, SystemError> {
        self.origins = self.origins.setup(|_| {})?;
        self.index = self.index.setup(|pond| {
            if pond.free() {
                self.free_list.push(pond.gene);
            }
        })?;
        // self.items = self.items.setup()?;

        Ok(self)
    }

    pub fn take_free(&mut self) -> Option<Gene> {
        self.free_list.pop(|_| true)
    }

    // 1. add: find a free slot or make a new pond
    // 2. del:

    pub fn half_empty_pond(
        &mut self, origin: &Origin,
    ) -> Result<Pond, SystemError> {
        let mut pond_gene = origin.first;
        let mut pond = Pond::default();
        loop {
            self.index.get(&pond_gene, &mut pond)?;
            // if pond.empty != 0
        }
    }

    pub fn add(
        &mut self, origene: &Gene, item: &mut T,
    ) -> Result<(), SystemError> {
        item.set_alive(true);

        let mut origin = Origin::default();
        self.origins.get(origene, &mut origin)?;

        // let mut pond_gene = origin.first;
        // let mut pond = Pond::default();
        // let child_index = loop {
        //     self.index.get(&pond_gene, &mut pond)?;
        //     if pond.stack != 0 && pond.empty != 0 {
        //         break;
        //     }
        //     // let mut ch = pond.children.iter().enumerate();
        //     // let Some((idx, _)) = ch.find(|(_, c)| c.gene != 0 && c.empty != 0)
        //     // else {
        //     //     if pond.next.is_none() {
        //     //         break None;
        //     //     }
        //     //     pond_gene = pond.next;
        //     //     continue;
        //     // };
        //     // break Some(idx);
        // };
        //
        // let mut buf = [T::default(); PAGE_SIZE];

        // if let Some(cdx) = child_index {
        //     let child = &mut pond.children[cdx];
        //     let pos = child.gene * T::N;
        //     child.alive += 1;
        //     child.empty -= 1;
        //     self.file.read_exact_at(buf.as_binary_mut(), pos)?;
        //     let item_gene = item.gene_mut();
        //     for (x, slot) in buf.iter_mut().enumerate() {
        //         let sg = slot.gene();
        //         if !slot.alive() && sg.iter < ITER_EXHAUSTION {
        //             item_gene.id = child.gene + x as u64;
        //             item_gene.iter = if sg.id != 0 { sg.iter + 1 } else { 0 };
        //             crate::utils::getrandom(&mut item_gene.pepper);
        //             item_gene.server = 69;
        //
        //             *slot = *item;
        //             break;
        //         }
        //     }
        //     self.file.write_all_at(buf.as_binary(), pos)?;
        //     self.index.set(&pond)?;
        //     origin.items += 1;
        //     self.origins.set(&origin)?;
        //     self.live += 1;
        //     return Ok(());
        // }
        //
        // if let Some(pond_gene) = self.take_dead() {
        //     self.index.get(&pond_gene, &mut pond)?;
        //     let child =
        //         pond.children.iter_mut().find(|c| c.gene != 0 && c.alive == 0);
        //     if child.is_none() {}
        // }

        Ok(())
    }

    // pub fn get(
    //     &mut self, gene: &Gene, brood: &mut Brood<T>,
    // ) -> Result<(), SystemError> {
    //     Ok(self.items.get(gene, brood)?)
    // }
    //
    // pub fn add(
    //     &mut self, origin_gene: &Gene, item: &mut T,
    // ) -> Result<(), SystemError> {
    //     let mut origin = Origin::default();
    //     self.origins.get(origin_gene, &mut origin)?;
    //
    //     let mut pond_gene = origin.first.clone();
    //     let mut pond = PondIndex::default();
    //     let brood = loop {
    //         self.index.get(&pond_gene, &mut pond)?;
    //         if let Some(child) = pond
    //             .children
    //             .iter()
    //             .find(|c| c.brood.is_some() && c.alive < PAGE_SIZE as u8)
    //         {
    //             break Some(child.brood.clone());
    //         }
    //         if pond.next.is_none() {
    //             break None;
    //         }
    //         pond_gene = pond.next;
    //     };
    //
    //     Ok(())
    // }
}
