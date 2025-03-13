use super::entity::{Entity, EntityDb, EntityInspector};
use crate::models::{
    Binary, DbHead, Gene, Performed, ShahMagic, ShahMagicDb, Task, TaskList,
};
use crate::{utils, AsStatic, Entity, BLOCK_SIZE};
use crate::{NotFound, ShahError, SystemError};

use std::os::unix::fs::FileExt;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

/// TOLERABLE CAPACITY DIFFERENCE
const TCD: u64 = 255;
const FREE_LIST_SIZE: usize = BLOCK_SIZE;
const SNAKE_MAGIC: ShahMagic = ShahMagic::new_const(ShahMagicDb::Snake as u16);
const SNAKE_VERSION: u16 = 1;

#[derive(Debug, Default, Clone, Copy)]
pub struct SnakeFree {
    gene: Gene,
    position: u64,
    capacity: u64,
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, Entity, Clone, Copy)]
pub struct SnakeHead {
    pub gene: Gene,
    pub capacity: u64,
    pub position: u64,
    pub length: u64,
    growth: u64,
    entity_flags: u32,
    #[flags(is_free)]
    pub flags: u32,
}

type SnakeIndexDb = EntityDb<SnakeHead, SnakeHead, (), &'static mut SnakeDb>;

#[derive(Debug)]
pub struct SnakeDb {
    file: File,
    pub live: u64,
    pub free: u64,
    pub free_list: Box<[Option<SnakeFree>; FREE_LIST_SIZE]>,
    pub index: SnakeIndexDb,
    name: String,
    ls: String,
    tasks: TaskList<1, Task<Self>>,
}

impl SnakeDb {
    pub fn new(path: &str) -> Result<Self, ShahError> {
        let data_path = Path::new("data/").join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path: {path}");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(data_path.join("data.snake.shah"))?;

        let mut db = Self {
            live: 0,
            free: 0,
            free_list: Box::new([None; BLOCK_SIZE]),
            file,
            index: SnakeIndexDb::new(&format!("{path}/index"), 0)?,
            ls: format!("<Snake {name} />"),
            name: name.to_string(),
            tasks: TaskList::new([Self::work_index]),
        };

        let dbs = db.as_static();
        let ei = EntityInspector::new(dbs, |mut db, head: &SnakeHead| {
            if head.is_free() {
                db.add_free(*head)?;
            }
            Ok(())
        });
        db.index.set_inspector(ei);

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        self.init_head()?;
        Ok(())
    }

    fn init_head(&mut self) -> Result<(), ShahError> {
        let fs = self.file_size()?;
        let mut head = DbHead::default();
        if fs < DbHead::N {
            head.init(SNAKE_MAGIC, 0, &self.name, SNAKE_VERSION);
            self.file.write_all_at(head.as_binary(), 0)?;
        } else {
            self.file.read_exact_at(head.as_binary_mut(), 0)?;
            head.check(&self.ls, SNAKE_MAGIC, 0, SNAKE_VERSION)?;
        }
        Ok(())
    }

    fn work_index(&mut self) -> Result<Performed, ShahError> {
        self.index.work()
    }

    pub fn work(&mut self) -> Result<Performed, ShahError> {
        self.tasks.start();
        while let Some(task) = self.tasks.next() {
            if task(self)?.0 {
                return Ok(Performed(true));
            }
        }
        Ok(Performed(false))
    }

    fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    fn take_free(
        &mut self, capacity: u64,
    ) -> Result<Option<SnakeFree>, ShahError> {
        let db_size = self.file_size()?;

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
                if free.capacity > capacity + TCD {
                    let val = SnakeFree {
                        position: free.position,
                        capacity,
                        gene: Gene::default(),
                    };

                    let mut disk = SnakeHead::default();
                    self.index.get(&free.gene, &mut disk)?;
                    if !disk.is_free() {
                        log::warn!("free is not even free on the disk :/ wtf");
                        return Ok(None);
                    }
                    if disk.capacity != free.capacity
                        || disk.position != free.position
                    {
                        log::warn!(
                            "invalid snake free_list: {disk:?} != {free:?}"
                        );
                        return Ok(None);
                    }

                    free.position += capacity;
                    free.capacity -= capacity;

                    disk.position = free.position;
                    disk.capacity = free.capacity;
                    self.index.set(&mut disk)?;

                    return Ok(Some(val));
                }

                let val = SnakeFree {
                    position: free.position,
                    capacity: free.capacity.max(capacity),
                    gene: free.gene,
                };

                if self.free > 0 {
                    self.free -= 1;
                }
                *opt_free = None;
                return Ok(Some(val));
            }

            if free.capacity < capacity {
                continue;
            }

            if free.capacity - capacity < TCD {
                if self.free > 0 {
                    self.free -= 1;
                }
                let val = *opt_free;
                *opt_free = None;
                return Ok(val);
            }

            let mut disk = SnakeHead::default();
            self.index.get(&free.gene, &mut disk)?;
            if !disk.is_free() {
                log::warn!("free is not even free :/ wtf");
                return Ok(None);
            }
            if disk.capacity != free.capacity || disk.position != free.position
            {
                log::warn!("invalid snake free_list: {disk:?} != {free:?}");
                return Ok(None);
            }

            disk.capacity -= capacity;
            self.index.set(&mut disk)?;
            free.capacity -= capacity;
            return Ok(Some(SnakeFree {
                gene: Default::default(),
                position: free.position + free.capacity,
                capacity,
            }));
        }

        Ok(None)
    }

    pub fn alloc(
        &mut self, capacity: u64, head: &mut SnakeHead,
    ) -> Result<(), ShahError> {
        if capacity == 0 {
            log::error!("{} alloc: capacity is zero", self.ls);
            return Err(SystemError::SnakeCapacityIsZero)?;
        }

        head.zeroed();
        head.set_alive(true);
        head.set_is_free(false);

        if let Some(free) = self.take_free(capacity)? {
            head.position = free.position;
            head.capacity = free.capacity;
            head.gene = free.gene;
        } else {
            head.position = self.file_size()?;
            if head.position < SnakeHead::N {
                head.position =
                    self.file.seek(SeekFrom::Start(SnakeHead::N))?;
            }
            head.capacity = capacity;
        }

        self.file.seek(SeekFrom::Start(head.position + head.capacity - 1))?;
        self.file.write_all(&[0u8])?;

        if let Err(e) = self.index.set(head) {
            e.not_found_ok()?;
            self.index.add(head)?;
        }

        Ok(())
    }

    fn check_offset(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64,
        buflen: usize,
    ) -> Result<usize, ShahError> {
        self.index.get(gene, head)?;
        if head.is_free() {
            return Err(NotFound::SnakeIsFree)?;
        }
        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);
        if offset >= head.capacity {
            log::error!(
                "{} check_offset: bad offset: {offset} >= {}",
                self.ls,
                head.capacity
            );
            return Err(SystemError::BadOffset)?;
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
    ) -> Result<(), ShahError> {
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
    ) -> Result<(), ShahError> {
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
    ) -> Result<(), ShahError> {
        self.index.get(gene, head)?;
        if head.is_free() {
            return Err(NotFound::SnakeIsFree)?;
        }
        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);
        if length > head.capacity {
            log::error!(
                "{} set_length: bad length: {length} >= {}",
                self.ls,
                head.capacity
            );
            return Err(SystemError::SnakeBadLength)?;
        }

        head.length = length;
        self.index.set(head)?;

        Ok(())
    }

    pub fn free(&mut self, gene: &Gene) -> Result<(), ShahError> {
        let mut head = SnakeHead::default();
        self.index.get(gene, &mut head)?;

        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);

        if head.is_free() {
            log::warn!("freeing already free");
            return Ok(());
        }

        head.set_is_free(true);
        self.index.set(&mut head)?;
        if let Err(e) = self.add_free(head) {
            log::warn!("add_free failed in free: {e:?}");
        }

        Ok(())
    }

    fn add_free(&mut self, mut head: SnakeHead) -> Result<(), ShahError> {
        if head.position == 0 || head.capacity == 0 || head.gene.id == 0 {
            log::warn!("invalid head into add_free");
            return Ok(());
        }

        let mut index = if self.free == 0 { 0 } else { FREE_LIST_SIZE };
        let mut travel = 0;
        // let mut round_two = false;
        // let mut round_two_index = 0usize;
        // log::warn!("add_free: {} | {}", head.gene.id, self.free);

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

            // log::trace!(
            //     "[{}] head: {} + {} == {}",
            //     head.gene.id,
            //     head.position,
            //     head.capacity,
            //     head.position + head.capacity
            // );
            // log::trace!(
            //     "[{}] free: {} + {} == {}",
            //     free.gene.id,
            //     free.position,
            //     free.capacity,
            //     free.position + free.capacity
            // );
            // log::trace!("round_two: {round_two}");

            if free.position + free.capacity == head.position {
                // log::info!("found before: round two: {round_two}");
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
                if self.free > 0 {
                    self.free -= 1;
                }

                // round_two = true;
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
                // log::info!("found after : round two: {round_two}");
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
                if self.free > 0 {
                    self.free -= 1;
                }

                // free.position = head.position;
                // free.capacity = head.capacity;
                // free.gene = head.gene;
                // self.index.set(&head)?;
                // head.position = free.position;
                // head.capacity = free.capacity;

                // round_two = true;
                fdx = 0;
                travel = 0;
                continue;
            }

            fdx += 1;
        }

        head.set_is_free(true);
        head.set_alive(true);
        self.index.set(&mut head)?;

        // log::error!("setting a free at: {index} | round_two: {round_two}");
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
