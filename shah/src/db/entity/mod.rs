mod face;
mod koch;

pub use face::*;
pub use koch::*;

use crate::models::*;
use crate::*;

use std::path::Path;
use std::{
    fmt::Debug,
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
};

const META_OFFSET: u64 = EntityHead::N + EntityKochProgress::N;
const ENTITY_MAGIC: ShahMagic =
    ShahMagic::new_const(ShahMagicDb::Entity as u16);

#[crate::model]
struct EntityHead {
    db_head: DbHead,
    item_size: u64,
    schema: [u8; 4096],
}

#[crate::model]
struct EntityKochProgress {
    total: u64,
    progress: u64,
}

#[derive(Debug)]
pub struct EntityCount {
    pub alive: u64,
    pub total: u64,
}

#[derive(Debug, Default)]
struct SetupTask {
    total: u64,
    progress: u64,
}

impl SetupTask {
    fn end(&mut self) {
        self.progress = self.total;
    }
}

impl Iterator for SetupTask {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        if self.progress >= self.total {
            return None;
        }

        let id = self.progress;
        self.progress += 1;
        Some(id)
    }
}

type EntityTask<T> = fn(&mut T) -> Result<bool, ShahError>;

#[derive(Debug)]
pub struct EntityDb<
    T: EntityItem + EntityKochFrom<Old, State>,
    Old: EntityItem = T,
    State: Debug = (),
> {
    pub file: File,
    pub live: u64,
    pub dead_list: DeadList<GeneId, BLOCK_SIZE>,
    iteration: u16,
    name: String,
    koch: Option<EntityKoch<T, Old, State>>,
    setup_task: SetupTask,
    tasks: TaskList<2, EntityTask<Self>>,
    ls: String,
}

/// if an io operation was performed check for order's
/// if no io operation's was performed then run another task
type Performed = bool;

impl<
        State: Debug,
        T: EntityItem + EntityKochFrom<Old, State>,
        Old: EntityItem,
    > EntityDb<T, Old, State>
{
    pub fn new(path: &str, iteration: u16) -> Result<Self, ShahError> {
        let path = Path::new("data/").join(path);
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path: {path}");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&path)?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.join(format!("{name}.{iteration}.shah")))?;

        let tasks = [Self::work_koch, Self::work_setup_task];
        let mut db = Self {
            live: 0,
            dead_list: DeadList::<GeneId, BLOCK_SIZE>::new(),
            file,
            iteration,
            name: name.to_string(),
            koch: None,
            setup_task: SetupTask::default(),
            tasks: TaskList::new(tasks),
            ls: format!("<EntityDb {name}.{iteration} />"),
        };

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        self.init_head()?;

        self.live = 0;
        self.dead_list.clear();

        let file_size = self.file_size()?;
        if file_size < META_OFFSET {
            return Err(DbError::BadInit)?;
        }

        if file_size < META_OFFSET + T::N {
            self.file.seek(SeekFrom::Start(META_OFFSET + T::N - 1))?;
            self.file.write_all(&[0u8])?;
            return Ok(());
        }

        if file_size == META_OFFSET + T::N {
            return Ok(());
        }

        self.live = ((file_size - META_OFFSET) / T::N) - 1;

        self.setup_task.progress = 1;
        self.setup_task.total = self.live + 1;
        log::info!("{} init::setup_task {:?}", self.ls, self.setup_task);

        Ok(())
    }

    fn init_head(&mut self) -> Result<(), ShahError> {
        let mut head = EntityHead::default();
        if let Err(e) = self.file.read_exact_at(head.as_binary_mut(), 0) {
            if e.kind() != ErrorKind::UnexpectedEof {
                return Err(e)?;
            }

            head.item_size = T::N;

            let svec = T::shah_schema().encode();
            head.schema[0..svec.len()].clone_from_slice(&svec);

            head.db_head.magic = ENTITY_MAGIC;
            head.db_head.iteration = self.iteration;
            head.db_head.set_name(&self.name);

            self.file.write_all_at(head.as_binary(), 0)?;

            return Ok(());
        }

        if head.db_head.magic != ENTITY_MAGIC {
            log::error!(
                "{} head invalid db magic: {:?} != {ENTITY_MAGIC:?}",
                self.ls,
                head.db_head.magic
            );
            return Err(DbError::InvalidDbHead)?;
        }

        if head.db_head.iteration != self.iteration {
            log::error!(
                "{} head invalid iteration {} != {}",
                self.ls,
                head.db_head.iteration,
                self.iteration
            );
            return Err(DbError::InvalidDbHead)?;
        }

        if head.item_size != T::N {
            log::error!(
                "{} schema.item_size != current item size. {} != {}",
                self.ls,
                head.item_size,
                T::N
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        let schema = Schema::decode(&head.schema)?;
        if schema != T::shah_schema() {
            log::error!(
                "{} mismatch schema. did you forgot to update the iternation?",
                self.ls,
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        Ok(())
    }

    // fn Koch

    pub fn set_koch(&mut self, koch: EntityKoch<T, Old, State>) {
        koch.total;
        self.koch = Some(koch);
    }

    // pub fn tasks<'t, 'edb: 't>(&'edb mut self) -> Result<Vec<Box<dyn Task + 't>>, ShahError> {
    //     let file_size = self.file_size()?;
    //     if file_size < META_OFFSET + T::N {
    //         return Err(DbError::BadInit)?;
    //     }
    //
    //     Ok(vec![
    //         Box::new(task::EntitySetupTask {
    //             total: self.live,
    //             progress: 0,
    //             db: self,
    //         }),
    //         Box::new(task::EntityKochTask {}),
    //     ])
    // }

    pub fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    // pub fn old_setup<F>(mut self, mut f: F) -> Result<Self, ShahError>
    // where
    //     F: FnMut(&mut Self, &T),
    // {
    //     self.live = 0;
    //     self.dead_list.clear();
    //     let file_size = self.file_size()?;
    //     if file_size < META_OFFSET {
    //         return Err(DbError::BadInit)?;
    //     }
    //
    //     let mut entity = T::default();
    //
    //     if file_size < META_OFFSET + T::N {
    //         self.file.seek(SeekFrom::Start(META_OFFSET + T::N - 1))?;
    //         self.file.write_all(&[0u8])?;
    //         return Ok(self);
    //     }
    //
    //     if file_size == META_OFFSET + T::N {
    //         return Ok(self);
    //     }
    //
    //     self.live = ((file_size - META_OFFSET) / T::N) - 1;
    //     // return Ok(self);
    //
    //     self.file.seek(SeekFrom::Start(META_OFFSET + T::N))?;
    //     loop {
    //         match self.file.read_exact(entity.as_binary_mut()) {
    //             Ok(_) => {}
    //             Err(e) => match e.kind() {
    //                 ErrorKind::UnexpectedEof => break,
    //                 _ => Err(e)?,
    //             },
    //         }
    //
    //         if !entity.is_alive() {
    //             let gene = entity.gene();
    //             log::debug!("dead entity: {entity:?}");
    //             self.add_dead(gene);
    //         }
    //
    //         f(&mut self, &entity);
    //     }
    //
    //     Ok(self)
    // }

    fn work_koch(&mut self) -> Result<Performed, ShahError> {
        let Some(koch) = &mut self.koch else {
            return Ok(false);
        };

        log::info!("koch.from: {:?}", koch.from);

        Ok(true)
    }

    fn work_setup_task(&mut self) -> Result<Performed, ShahError> {
        if self.dead_list.is_full() {
            return Ok(false);
        }
        let Some(id) = self.setup_task.next() else {
            return Ok(false);
        };

        log::info!("work setup task id: {id}");

        let mut entity = T::default();

        self.seek_id(id)?;
        if let Err(e) = self.read(&mut entity) {
            e.not_found_ok()?;
            self.setup_task.end();
            log::warn!("read not found");
            return Ok(true);
        }

        if !entity.is_alive() {
            let gene = entity.gene();
            log::debug!("dead entity: {}", gene.id);
            self.add_dead(gene);
        }

        Ok(true)
    }

    pub fn work(&mut self) -> Result<Performed, ShahError> {
        self.tasks.start();
        while let Some(task) = self.tasks.next() {
            if task(self)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn seek_id(&mut self, id: GeneId) -> Result<(), ShahError> {
        // if id == 0 {
        //     log::warn!("gene id is zero");
        //     return Err(NotFound::ZeroGeneId)?;
        // }

        // let db_size = self.file_size()?;
        let pos = META_OFFSET + id * T::N;

        // if pos > db_size - T::N {
        //     log::warn!("invalid position: {pos}/{db_size}");
        //     return Err(NotFound::GeneIdNotInDatabase)?;
        // }

        self.file.seek(SeekFrom::Start(pos))?;

        Ok(())
    }

    pub fn read(&mut self, entity: &mut T) -> Result<(), ShahError> {
        match self.file.read_exact(entity.as_binary_mut()) {
            Ok(_) => {}
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => Err(NotFound::OutOfBounds)?,
                _ => Err(e)?,
            },
        }

        Ok(())
    }

    pub fn get(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        gene.validate()?;

        self.seek_id(gene.id)?;
        self.read(entity)?;

        let gene = entity.gene();
        if gene.id == 0 {
            // if let Some(koch) = self.koch {
            //     // koch.from.get(gene, entity)
            // }
        }
        gene.check(gene)?;

        if !entity.is_alive() {
            return Err(NotFound::EntityNotAlive)?;
        }

        Ok(())
    }

    pub fn new_gene_id(&mut self) -> Result<GeneId, ShahError> {
        let pos = self.file.seek(SeekFrom::End(0))?;
        if pos < META_OFFSET + T::N {
            return Ok(1);
        }

        let db_pos = pos - META_OFFSET;
        let (id, offset) = (db_pos / T::N, db_pos % T::N);
        if offset != 0 {
            log::warn!(
                "{} id: {id} | new-gene-id bad offset: {offset}",
                self.ls
            );
            return Ok(id);
        }

        Ok(id)
    }

    pub fn new_gene(&mut self) -> Result<Gene, ShahError> {
        let mut gene = Gene { id: self.take_dead_id(), ..Default::default() };
        crate::utils::getrandom(&mut gene.pepper);
        gene.server = 0;
        gene.iter = 0;

        if gene.id != 0 && self.seek_id(gene.id).is_ok() {
            let mut og = Gene::default();
            if self.file.read_exact(og.as_binary_mut()).is_ok() {
                #[allow(clippy::collapsible_if)]
                if og.iter < ITER_EXHAUSTION {
                    gene.iter = og.iter + 1;
                    return Ok(gene);
                }
            }
        }

        gene.id = self.new_gene_id()?;

        Ok(gene)
    }

    pub fn add(&mut self, entity: &mut T) -> Result<(), ShahError> {
        entity.set_alive(true);
        if entity.gene().id == 0 {
            entity.gene_mut().clone_from(&self.new_gene()?);
        }

        let id = entity.gene().id;
        self.seek_id(id)?;
        self.file.write_all(entity.as_binary_mut())?;
        self.live += 1;

        Ok(())
    }

    pub fn count(&mut self) -> Result<EntityCount, ShahError> {
        let db_size = self.file_size()?;
        let total = db_size / T::N - 1;
        Ok(EntityCount { total, alive: self.live })
    }

    pub fn take_dead_id(&mut self) -> GeneId {
        self.dead_list.pop(|_| true).unwrap_or_default()
    }

    pub fn add_dead(&mut self, gene: &Gene) {
        self.live -= 1;

        if gene.iter >= ITER_EXHAUSTION {
            return;
        }

        self.dead_list.push(gene.id);
    }

    pub fn set(&mut self, entity: &T) -> Result<(), ShahError> {
        if !entity.is_alive() {
            return Err(NotFound::DeadSet)?;
        }

        let mut old_entity = T::default();
        self.get(entity.gene(), &mut old_entity)?;

        self.seek_id(entity.gene().id)?;
        self.file.write_all(entity.as_binary())?;

        Ok(())
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        self.get(gene, entity)?;

        entity.set_alive(false);

        self.seek_id(gene.id)?;
        self.file.write_all(entity.as_binary())?;

        self.add_dead(gene);

        Ok(())
    }

    pub fn list(
        &mut self, page: u64, result: &mut [T; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        self.seek_id(page * PAGE_SIZE as u64 + 1)?;
        let size = self.file.read(result.as_binary_mut())?;
        let count = size / T::S;
        if count != PAGE_SIZE {
            for item in result.iter_mut().skip(count) {
                item.zeroed()
            }
        }

        Ok(count)
    }
}
