use super::entity::{Entity, EntityDb};
use crate::{error::SystemError, Binary, Gene, BLOCK_SIZE};
use shah_macros::Entity;
use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
};

/// TOLERABLE CAPACITY DIFFERENCE
const TCD: u64 = 255;
const FREE_LIST_SIZE: usize = BLOCK_SIZE;

// FIXME: the free snakes are not handled correctly

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
    pub free_list: Box<[Option<SnakeFree>; FREE_LIST_SIZE]>,
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
            free_list: Box::new([None; BLOCK_SIZE]),
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
        self.index.dead_list.clear();
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
                if !head.is_alive() {
                    self.index.live -= 1;
                    log::debug!("dead head: {head:?}");
                    self.index.add_dead(&head.gene);
                } else if head.is_free() {
                    if let Err(e) = self.add_free(*head) {
                        log::warn!("add_free failed in setup: {e:?}");
                    }
                }
            }
        }

        Ok(self)
    }

    fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    fn take_free(
        &mut self, capacity: u64,
    ) -> Result<Option<SnakeFree>, SystemError> {
        let db_size = self.db_size()?;

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

            if free.position + free.capacity == db_size {
                self.free -= 1;
                let val = SnakeFree {
                    position: free.position,
                    gene: free.gene,
                    capacity,
                };
                *opt_free = None;
                return Ok(Some(val));
            }

            if free.capacity >= capacity {
                if free.capacity - capacity < TCD {
                    self.free -= 1;
                    let val = *opt_free;
                    *opt_free = None;
                    return Ok(val);
                }

                let mut old = SnakeHead::default();
                self.index.get(&free.gene, &mut old)?;
                if !old.is_free() {
                    log::warn!("free is not even free :/ wtf");
                    return Ok(None);
                }
                if old.capacity != free.capacity
                    || old.position != free.position
                {
                    log::warn!("invalid snake free_list: {old:?} != {free:?}");
                    return Ok(None);
                }

                old.capacity -= capacity;
                self.index.set(&old)?;
                free.capacity -= capacity;
                return Ok(Some(SnakeFree {
                    gene: Default::default(),
                    position: free.position + free.capacity,
                    capacity,
                }));
            }
        }

        Ok(None)
    }

    pub fn alloc(
        &mut self, capacity: u64, head: &mut SnakeHead,
    ) -> Result<(), SystemError> {
        if capacity == 0 {
            return Err(SystemError::SnakeCapacityIsZero);
        }

        head.zeroed();
        head.set_alive(true);
        head.set_free(false);

        if let Some(free) = self.take_free(capacity)? {
            println!("take dead: {free:?}");
            head.position = free.position;
            head.capacity = free.capacity;
            if free.gene.is_some() {
                head.gene = free.gene;
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

        if head.gene.is_some() {
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
        if head.is_free() {
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

        // self.file.seek(SeekFrom::Start(head.position + head.capacity - 1))?;
        // self.file.write_all(&[0u8])?;

        self.file.seek(SeekFrom::Start(head.position + offset))?;
        self.file.write_all(&data[..len])?;

        Ok(())
    }

    pub fn read(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64,
        data: &mut [u8],
    ) -> Result<(), SystemError> {
        let len = self.check_offset(gene, head, offset, data.len())?;
        // log::info!(
        //     "read len: {len} - offset: {offset} - data len: {} - head: {head:#?}",
        //     data.len()
        // );

        self.file.seek(SeekFrom::Start(head.position + offset))?;
        self.file.read_exact(&mut data[..len])?;

        Ok(())
    }

    pub fn set_length(
        &mut self, gene: &Gene, head: &mut SnakeHead, length: u64,
    ) -> Result<(), SystemError> {
        self.index.get(gene, head)?;
        if head.is_free() {
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

    pub fn free(&mut self, gene: &Gene) -> Result<(), SystemError> {
        let mut head = SnakeHead::default();
        self.index.get(gene, &mut head)?;

        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);

        if head.is_free() {
            log::warn!("freeing already free");
            return Ok(());
        }

        head.set_free(true);
        self.index.set(&head)?;
        if let Err(e) = self.add_free(head) {
            log::warn!("add_free failed in free: {e:?}");
        }

        Ok(())
    }

    fn add_free(&mut self, mut head: SnakeHead) -> Result<(), SystemError> {
        if head.position == 0 || head.capacity == 0 || head.gene.is_none() {
            log::warn!("invalid head into add_free");
            return Ok(());
        }

        let mut index = if self.free == 0 { 0 } else { FREE_LIST_SIZE };
        let mut travel = 0;
        let mut round_two = false;
        // let mut round_two_index = 0usize;
        log::warn!("add_free: {} | {}", head.gene.id, self.free);

        let mut fdx = 0usize;
        while fdx < FREE_LIST_SIZE {
            if travel > self.free {
                break;
            }
            let slot = &mut self.free_list[fdx];
            let Some(free) = slot else {
                if index == FREE_LIST_SIZE {
                    index = fdx;
                }
                fdx += 1;
                continue;
            };
            travel += 1;

            assert_ne!(free.position, 0);
            assert_ne!(free.capacity, 0);
            assert_ne!(free.gene.id, 0);

            log::trace!(
                "[{}] head: {} + {} == {}",
                head.gene.id,
                head.position,
                head.capacity,
                head.position + head.capacity
            );
            log::trace!(
                "[{}] free: {} + {} == {}",
                free.gene.id,
                free.position,
                free.capacity,
                free.position + free.capacity
            );
            log::trace!("round_two: {round_two}");

            if free.position + free.capacity == head.position {
                log::info!("found before: round two: {round_two}");
                let mut disk = SnakeHead::default();
                self.index.get(&free.gene, &mut disk)?;

                if disk.capacity != free.capacity
                    || disk.position != free.position
                {
                    log::warn!("disk != free: {disk:?} - {free:?}");
                    return Ok(());
                }

                head.position = free.position;
                head.capacity += free.capacity;

                self.index.del(&disk.gene, &mut SnakeHead::default())?;
                *slot = None;
                self.free -= 1;

                round_two = true;
                fdx = 0;
                travel = 0;
                continue;

                // old.capacity += head.capacity;
                // free.capacity += head.capacity;
                // if let Err(e) = self.index.set(&old) {
                //     log::warn!("set old free: {e:?} - {:?}", free.gene);
                //     return;
                // }
                // if let Err(e) =
                //     self.index.del(&head.gene, &mut SnakeHead::default())
                // {
                //     log::warn!("del head: {e:?} - {:?}", head.gene);
                //     return;
                // }
                //
                // log::info!("go for round two");
                // round_two = true;
                // fdx = 0;
                // head.position = free.position;
                // head.capacity = free.capacity;
                // head.gene = free.gene;
                // travel = 0;
                // continue;
            }

            if head.position + head.capacity == free.position {
                log::info!("found after : round two: {round_two}");
                let mut disk = SnakeHead::default();
                self.index.get(&free.gene, &mut disk)?;

                if disk.capacity != free.capacity
                    || disk.position != free.position
                {
                    log::warn!("disk != free: {disk:?} - {free:?}");
                    return Ok(());
                }

                head.capacity += free.capacity;

                self.index.del(&disk.gene, &mut SnakeHead::default())?;
                *slot = None;
                self.free -= 1;

                // free.position = head.position;
                // free.capacity = head.capacity;
                // free.gene = head.gene;
                // self.index.set(&head)?;
                // head.position = free.position;
                // head.capacity = free.capacity;

                round_two = true;
                fdx = 0;
                travel = 0;
                continue;
            }

            fdx += 1;
        }

        head.set_free(true);
        head.set_alive(true);
        self.index.set(&head)?;

        log::error!("setting a free at: {index} | round_two: {round_two}");
        if index < FREE_LIST_SIZE {
            let opt_free = &mut self.free_list[index];
            if opt_free.is_some() {
                log::warn!("invalid free index. item space occupied: {index}");
                return Ok(());
            }
            *opt_free = Some(SnakeFree {
                position: head.position,
                capacity: head.capacity,
                gene: head.gene,
            });
            self.free += 1;
        }

        Ok(())
    }
}
