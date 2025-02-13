mod face;
mod koch;
mod meta;

pub use face::*;
pub use koch::*;
pub use meta::*;

use crate::models::*;
use crate::*;

use std::path::Path;
use std::{
    fmt::Debug,
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
};

#[derive(Debug)]
pub struct EntityCount {
    pub alive: u64,
    pub total: u64,
}

#[derive(Debug, Default)]
struct SetupTask {
    total: u64,
    prog: u64,
}

impl SetupTask {
    fn end(&mut self) {
        self.prog = self.total;
    }
}

impl Iterator for SetupTask {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        if self.prog >= self.total {
            return None;
        }

        let id = self.prog;
        self.prog += 1;
        Some(id)
    }
}

type EntityTask<T> = fn(&mut T) -> Result<bool, ShahError>;

#[derive(Debug)]
pub struct EntityDb<
    T: EntityItem + EntityKochFrom<O, S>,
    O: EntityItem = T,
    S = (),
> {
    pub file: File,
    pub live: u64,
    pub dead_list: DeadList<GeneId, BLOCK_SIZE>,
    iteration: u16,
    name: String,
    koch: Option<EntityKoch<T, O, S>>,
    koch_prog: EntityKochProg,
    setup_task: SetupTask,
    tasks: TaskList<2, EntityTask<Self>>,
    ls: String,
    inspector: Option<fn(&mut Self, &T)>,
}

/// if an io operation was performed check for order's
/// if no io operation's was performed then run another task
type Performed = bool;

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem> EntityDb<T, O, S> {
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
            koch_prog: EntityKochProg::default(),
            setup_task: SetupTask::default(),
            tasks: TaskList::new(tasks),
            ls: format!("<EntityDb {name}.{iteration} />"),
            inspector: None,
        };

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        self.init_head()?;
        self.koch_prog_get()?;

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

        self.setup_task.prog = 1;
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

        head.check::<T>(self.iteration, &self.ls)?;

        Ok(())
    }

    fn koch_prog_get(&mut self) -> Result<(), ShahError> {
        let buf = self.koch_prog.as_binary_mut();
        if let Err(e) = self.file.read_exact_at(buf, EntityHead::N) {
            if e.kind() != ErrorKind::UnexpectedEof {
                return Err(e)?;
            }

            self.koch_prog = EntityKochProg::default();
            self.koch_prog_set()?;
        }

        Ok(())
    }

    fn koch_prog_set(&mut self) -> Result<(), ShahError> {
        self.file.write_all_at(self.koch_prog.as_binary(), EntityHead::N)?;
        Ok(())
    }

    pub fn set_koch(&mut self, koch: EntityKoch<T, O, S>) {
        self.setup_task.total = self.koch_prog.prog;
        self.setup_task.prog = 0;

        log::debug!("{} set_koch setup_task: {:?}", self.ls, self.setup_task);

        self.koch_prog.total = koch.total;

        log::debug!("{} set_koch koch_prog: {:?}", self.ls, self.koch_prog);

        self.koch = Some(koch);
    }

    pub fn set_inspector(&mut self, inspector: fn(&mut Self, &T)) {
        self.inspector = Some(inspector);
    }

    pub fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    fn inspection(&mut self, entity: &T) {
        if !entity.is_alive() {
            let gene = entity.gene();
            log::debug!("{} inspector dead entity: {}", self.ls, gene.id);
            self.add_dead(gene);
        }

        if let Some(inspector) = self.inspector {
            inspector(self, entity)
        }
    }

    fn work_koch(&mut self) -> Result<Performed, ShahError> {
        if self.koch.is_none() {
            return Ok(false);
        }

        let mut performed = false;
        for _ in 0..10 {
            let Some(id) = self.koch_prog.next() else { break };
            let Some(koch) = self.koch.as_mut() else { break };

            let item = match koch.get_id(id) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("{} koch.get_id({id}): {e:?}", self.ls);
                    e.not_found_ok()?;
                    self.koch_prog.end();
                    break;
                }
            };
            self.file.write_all_at(item.as_binary(), Self::id_pos(id))?;

            log::debug!("koched: {:?}", item.gene());
            self.inspection(&item);
            performed = true;
        }

        if performed {
            self.koch_prog_set()?;
        }

        Ok(performed)
    }

    fn work_setup_task(&mut self) -> Result<Performed, ShahError> {
        log::debug!("work_setup_task");
        if self.dead_list.is_full() {
            return Ok(false);
        }

        let mut entity = T::default();
        let mut performed = false;
        for _ in 0..10 {
            let Some(id) = self.setup_task.next() else { break };
            performed = true;
            if let Err(e) = self.read_at(&mut entity, Self::id_pos(id)) {
                e.not_found_ok()?;
                self.setup_task.end();
                log::warn!(
                    "{} work_setup_task read_at not found {id}",
                    self.ls
                );
                break;
            }

            self.inspection(&entity);
        }

        Ok(performed)
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

    pub fn id_pos(id: GeneId) -> u64 {
        META_OFFSET + id * T::N
    }

    pub fn seek_id(&mut self, id: GeneId) -> Result<(), ShahError> {
        // if id == 0 {
        //     log::warn!("gene id is zero");
        //     return Err(NotFound::ZeroGeneId)?;
        // }

        // let db_size = self.file_size()?;

        // if pos > db_size - T::N {
        //     log::warn!("invalid position: {pos}/{db_size}");
        //     return Err(NotFound::GeneIdNotInDatabase)?;
        // }

        self.file.seek(SeekFrom::Start(Self::id_pos(id)))?;

        Ok(())
    }

    pub fn read_at(
        &mut self, entity: &mut T, pos: u64,
    ) -> Result<(), ShahError> {
        match self.file.read_exact_at(entity.as_binary_mut(), pos) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => Err(NotFound::OutOfBounds)?,
                _ => Err(e)?,
            },
        }
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
        if self.live > 0 {
            self.live -= 1;
        }

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
