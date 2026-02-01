mod api;
mod free;

use super::entity::{EntityDb, EntityInspector};
use crate::config::ShahConfig;
use crate::db::entity::EntityFlags;
use crate::models::{
    Binary, DbHead, Gene, Performed, ShahMagic, ShahMagicDb, Task, TaskList,
    Worker,
};
use crate::{AsStatic, BLOCK_SIZE, Entity, utils};
use crate::{NotFound, ShahError, SystemError};
use std::os::unix::fs::FileExt;
use std::{
    fs::File,
    io::{Seek, SeekFrom},
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

#[cfg_attr(feature = "serde", shah::flags(inner = u8, serde = true))]
#[cfg_attr(not(feature = "serde"), shah::flags(inner = u8, serde = false))]
pub struct SnakeFlags {
    is_free: bool,
}

#[derive(crate::ShahSchema)]
#[crate::model]
#[derive(Debug, Entity)]
pub struct SnakeHead {
    pub gene: Gene,
    pub capacity: u64,
    pub position: u64,
    pub length: u64,
    growth: u64,
    entity_flags: EntityFlags,
    pub flags: SnakeFlags,
    _pad: [u8; 6],
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
        let conf = ShahConfig::get();
        let data_path = conf.data_dir.join(path);
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
            ls: format!("<Snake {path} />"),
            name: name.to_string(),
            tasks: TaskList::new([Self::work_index]),
        };

        let dbs = db.as_static();
        let ei = EntityInspector::new(dbs, |mut db, head: &SnakeHead| {
            if head.flags.is_free() {
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

    fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    fn check_offset(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64,
        buflen: usize,
    ) -> Result<usize, ShahError> {
        self.index.get(gene, head)?;
        if head.flags.is_free() {
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
}

impl Worker<1> for SnakeDb {
    fn tasks(&mut self) -> &mut TaskList<1, Task<Self>> {
        &mut self.tasks
    }
}
