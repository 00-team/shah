use super::entity::{Entity, EntityDb};
use crate::{error::SystemError, Binary, Gene};
use shah_macros::Entity;
use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
};

/// TOLERABLE CAPACITY DIFFERENCE
const TCD: u64 = 255;

#[derive(Debug, Default, Clone, Copy)]
pub struct SnakeDead {
    gene: Gene,
    position: u64,
    capacity: u64,
}

#[shah::model]
#[derive(Debug, Entity, Clone, Copy)]
pub struct SnakeHead {
    pub gene: Gene,
    pub capacity: u64,
    pub position: u64,
    pub length: u64,
    #[entity_flags]
    pub entity_flags: u8,
    #[flags(free)]
    pub flags: u8,
    _pad: [u8; 6],
}

#[derive(Debug)]
pub struct SnakeDb {
    pub file: File,
    pub live: u64,
    pub dead: u64,
    pub dead_list: [SnakeDead; 4096],
    pub index: EntityDb<SnakeHead>,
}

impl SnakeDb {
    pub fn new(name: &str) -> Result<Self, SystemError> {
        std::fs::create_dir_all("data/")?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.snake.bin"))?;

        let db = Self {
            live: 0,
            dead: 0,
            dead_list: [SnakeDead::default(); 4096],
            file,
            index: EntityDb::<SnakeHead>::new(&format!("{name}.index"))?,
        };

        Ok(db)
    }

    pub fn setup(mut self) -> Self {
        self.index = self
            .index
            .setup(|head| {
                log::info!("this is the head: {head:?}");
                // self.dead += 1;
            })
            .expect("snake index setup");

        self
    }

    pub fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub fn take_free(&mut self, capacity: u64) -> Option<SnakeDead> {
        let mut travel = 0;
        for dead in self.dead_list.iter_mut() {
            travel += 1;
            if travel > self.dead {
                break;
            }

            if dead.position == 0 || dead.capacity == 0 {
                continue;
            };

            if dead.capacity >= capacity {
                if dead.capacity - capacity < TCD {
                    self.dead -= 1;
                    let sd = dead.clone();
                    dead.as_binary_mut().fill(0);
                    return Some(sd);
                }

                let mut old = SnakeHead::default();
                if let Err(e) = self.index.get(&dead.gene, &mut old) {
                    log::warn!("error reading dead snake head: {e:?}");
                    return None;
                }
                if old.alive() {
                    log::warn!("dead is not even dead :/ wtf");
                    return None;
                }
                if old.capacity != dead.capacity
                    || old.position != dead.position
                {
                    log::warn!("invalid snake dead_list: {old:?} != {dead:?}");
                    return None;
                }

                old.capacity -= capacity;
                if let Err(e) = self.index.set(&old) {
                    log::warn!("error updating old dead snake: {e:?}");
                    return None;
                };
                dead.capacity -= capacity;
                return Some(SnakeDead {
                    gene: Default::default(),
                    position: dead.position + dead.capacity,
                    capacity,
                });
            }
        }

        None
    }

    pub fn alloc(&mut self, capacity: u64) -> Result<SnakeHead, SystemError> {
        if capacity == 0 {
            return Err(SystemError::SnakeCapacityIsZero);
        }

        let mut head = SnakeHead::default();
        head.set_alive(true);

        if let Some(dead) = self.take_free(capacity) {
            println!("take dead: {dead:?}");
            head.position = dead.position;
            head.capacity = dead.capacity;
            if dead.gene.id != 0 {
                head.gene = dead.gene;
            }
        } else {
            head.position = self.db_size()?;
            head.capacity = capacity;
            self.file.seek_relative((capacity - 1) as i64)?;
            self.file.write_all(&[0u8])?;
        }

        if head.gene.id != 0 {
            self.index.set(&head)?;
        }

        Ok(head)
    }

    // pub fn add_dead(&mut self, head: &SnakeHead) {
    //     self.live -= 1;
    //     if self.dead as usize >= self.dead_list.len() {
    //         return;
    //     }
    //
    //     if head.position == 0 || head.capacity == 0 {
    //         log::warn!("adding in invalid SnakeDead to dead_list: {head:?}");
    //         return;
    //     }
    //
    //     // // combined size of head + tail = snake :)
    //     // let size = SnakeHead::N + new.capacity;
    //     //
    //     // for old in self.dead_list.iter_mut() {
    //     //     if old.position + old.capacity == new.position {}
    //     // }
    // }
}
