use super::entity::{Entity, EntityDb};
use crate::{error::SystemError, Binary, Gene};
use shah_macros::Entity;
use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
};

/// TOLERABLE CAPACITY DIFFERENCE
const TCD: u64 = 255;

#[derive(Debug, Default, Clone, Copy)]
pub struct SnakeFree {
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
    pub entity_flags: u32,
    #[flags(free)]
    pub flags: u32,
}

#[derive(Debug)]
pub struct SnakeDb {
    pub file: File,
    pub live: u64,
    pub free: u64,
    pub free_list: [Option<SnakeFree>; 4096],
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
            free: 0,
            free_list: [None; 4096],
            file,
            index: EntityDb::<SnakeHead>::new(&format!("{name}.index"))?,
        };

        Ok(db)
    }

    pub fn setup(mut self) -> Result<Self, SystemError> {
        if self.db_size()? < SnakeHead::N {
            self.file.seek(SeekFrom::Start(SnakeHead::N - 1))?;
            self.file.write_all(&[0u8])?;
        }
        // this is so fucking annoying because of borrow rules
        // i have to setup index manually
        self.index.live = 0;
        self.index.dead = 0;
        self.index.dead_list.fill(0);
        let index_db_size = self.index.db_size()?;
        let mut head = SnakeHead::default();
        let buf = head.as_binary_mut();

        if index_db_size < SnakeHead::N {
            self.index.file.seek(SeekFrom::Start(SnakeHead::N - 1))?;
            self.index.file.write_all(&[0u8])?;
            return Ok(self);
        }

        if index_db_size == SnakeHead::N {
            return Ok(self);
        }

        self.index.live = (index_db_size / SnakeHead::N) - 1;
        self.index.file.seek(SeekFrom::Start(SnakeHead::N))?;
        loop {
            match self.index.file.read_exact(buf) {
                Ok(_) => {}
                Err(e) => match e.kind() {
                    ErrorKind::UnexpectedEof => break,
                    _ => Err(e)?,
                },
            }
            {
                let head = SnakeHead::from_binary_mut(buf);
                if !head.alive() {
                    log::debug!("dead head: {head:?}");
                    self.index.add_dead(&head.gene);
                } else if head.free() {
                    self.add_free(head);
                }
            }
        }

        Ok(self)
    }

    fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    fn take_free(&mut self, capacity: u64) -> Option<SnakeFree> {
        let mut travel = 0;
        for opt_free in self.free_list.iter_mut() {
            if travel >= self.free {
                break;
            }
            let Some(free) = opt_free else { continue };
            travel += 1;

            assert_ne!(free.position, 0);
            assert_ne!(free.capacity, 0);
            assert_ne!(free.gene.id, 0);

            if free.capacity >= capacity {
                if free.capacity - capacity < TCD {
                    self.free -= 1;
                    let val = *opt_free;
                    *opt_free = None;
                    return val;
                }

                let mut old = SnakeHead::default();
                if let Err(e) = self.index.get(&free.gene, &mut old) {
                    log::warn!("error reading free snake head: {e:?}");
                    return None;
                }
                if !old.free() {
                    log::warn!("free is not even free :/ wtf");
                    return None;
                }
                if old.capacity != free.capacity
                    || old.position != free.position
                {
                    log::warn!("invalid snake free_list: {old:?} != {free:?}");
                    return None;
                }

                old.capacity -= capacity;
                if let Err(e) = self.index.set(&old) {
                    log::warn!("error updating old dead snake: {e:?}");
                    return None;
                };
                free.capacity -= capacity;
                return Some(SnakeFree {
                    gene: Default::default(),
                    position: free.position + free.capacity,
                    capacity,
                });
            }
        }

        None
    }

    pub fn alloc(
        &mut self, capacity: u64, head: &mut SnakeHead,
    ) -> Result<(), SystemError> {
        if capacity == 0 {
            return Err(SystemError::SnakeCapacityIsZero);
        }

        head.set_alive(true);
        head.set_free(false);

        if let Some(dead) = self.take_free(capacity) {
            println!("take dead: {dead:?}");
            head.position = dead.position;
            head.capacity = dead.capacity;
            if dead.gene.id != 0 {
                head.gene = dead.gene;
            }
        } else {
            head.position = self.db_size()?;
            if head.position < SnakeHead::N {
                head.position =
                    self.file.seek(SeekFrom::Start(SnakeHead::N))?;
            }
            head.capacity = capacity;
            self.file.seek_relative((capacity - 1) as i64)?;
            self.file.write_all(&[0u8])?;
        }

        if head.gene.id != 0 {
            self.index.set(head)?;
        } else {
            self.index.add(head)?;
        }

        Ok(())
    }

    fn check_offset(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64,
        buflen: usize,
    ) -> Result<usize, SystemError> {
        self.index.get(gene, head)?;
        if head.free() {
            return Err(SystemError::SnakeIsFree);
        }
        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);
        if offset >= head.capacity {
            return Err(SystemError::BadOffset);
        }
        let len = if offset + (buflen as u64) > head.capacity {
            (head.capacity - offset) as usize
        } else {
            buflen
        };

        Ok(len)
    }

    pub fn write(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64, data: &[u8],
    ) -> Result<(), SystemError> {
        let len = self.check_offset(gene, head, offset, data.len())?;

        self.file.seek(SeekFrom::Start(head.position + offset))?;
        self.file.write_all(&data[..len])?;

        Ok(())
    }

    pub fn read(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64,
        data: &mut [u8],
    ) -> Result<(), SystemError> {
        let len = self.check_offset(gene, head, offset, data.len())?;

        self.file.seek(SeekFrom::Start(head.position + offset))?;
        self.file.read_exact(&mut data[..len])?;

        Ok(())
    }

    pub fn set_length(
        &mut self, gene: &Gene, head: &mut SnakeHead, length: u64,
    ) -> Result<(), SystemError> {
        self.index.get(gene, head)?;
        if head.free() {
            return Err(SystemError::SnakeIsFree);
        }
        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);
        if length > head.capacity {
            return Err(SystemError::SnakeBadLength);
        }

        head.length = length;
        self.index.set(head)?;

        Ok(())
    }

    pub fn free(
        &mut self, gene: &Gene, head: &mut SnakeHead,
    ) -> Result<(), SystemError> {
        self.index.get(gene, head)?;

        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);

        if head.free() {
            return Ok(());
        }

        head.set_free(true);
        self.index.set(head)?;
        self.add_free(head);

        Ok(())
    }

    fn add_free(&mut self, head: &mut SnakeHead) {
        if head.position == 0 || head.capacity == 0 || head.gene.id == 0 {
            return;
        }

        let mut index = 0;
        let mut travel = 0;
        for opt_free in self.free_list.iter_mut() {
            if travel >= self.free {
                break;
            }
            let Some(free) = opt_free else { continue };
            travel += 1;

            assert_ne!(free.position, 0);
            assert_ne!(free.capacity, 0);
            assert_ne!(free.gene.id, 0);

            if free.position + free.capacity == head.position {
                let mut old = SnakeHead::default();
                if let Err(e) = self.index.get(&free.gene, &mut old) {
                    log::warn!("get old free: {e:?} - {:?}", free.gene);
                    return;
                }
                if old.capacity != free.capacity
                    || old.position != free.position
                {
                    log::warn!("old != free: {old:?} - {free:?}");
                    return;
                }
                old.capacity += head.capacity;
                free.capacity += head.capacity;
                if let Err(e) = self.index.set(&old) {
                    log::warn!("set old free: {e:?} - {:?}", free.gene);
                    return;
                }
                head.set_alive(false);
                if let Err(e) = self.index.set(head) {
                    log::warn!("set head: {e:?} - {:?}", head.gene);
                    return;
                }

                return;
            }

            if head.position + head.capacity == free.position {
                let mut old = SnakeHead::default();
                if let Err(e) = self.index.get(&free.gene, &mut old) {
                    log::warn!("get old free: {e:?} - {:?}", free.gene);
                    return;
                }
                if old.capacity != free.capacity
                    || old.position != free.position
                {
                    log::warn!("old != free: {old:?} - {free:?}");
                    return;
                }
                old.set_alive(false);
                if let Err(e) = self.index.set(&old) {
                    log::warn!("set old free: {e:?} - {:?}", free.gene);
                    return;
                }

                head.capacity += free.capacity;

                free.position = head.position;
                free.capacity = head.capacity;
                free.gene = head.gene;
                if let Err(e) = self.index.set(head) {
                    log::warn!("set head: {e:?} - {:?}", head.gene);
                    return;
                }

                return;
            }

            index += 1;
        }

        if index < self.free_list.len() {
            let opt_free = &mut self.free_list[index];
            if opt_free.is_some() {
                log::warn!("invalid free index. item space occupied: {index}");
                return;
            }
            *opt_free = Some(SnakeFree {
                position: head.position,
                capacity: head.capacity,
                gene: head.gene,
            });
            self.free += 1;
            head.set_free(true);
            head.set_alive(true);
            if let Err(e) = self.index.set(head) {
                log::warn!("set head: {e:?} - {:?}", head.gene);
            }
        }
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
