mod face;
mod koch;
mod meta;

pub use face::*;
pub use koch::*;
pub use meta::*;

use crate::models::*;
use crate::*;

use std::cell::{RefCell, RefMut};
use std::marker::PhantomData;
use std::ops::AddAssign;
use std::path::Path;
use std::{
    fmt::Debug,
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
    os::unix::fs::FileExt,
};

#[derive(Debug)]
pub struct EntityCount {
    pub alive: GeneId,
    pub total: GeneId,
}

#[derive(Debug, Default)]
struct SetupProg {
    total: GeneId,
    prog: GeneId,
}

id_iter!(SetupProg);

type EntityInspectorFn<T, S> = fn(RefMut<S>, &T) -> Result<(), ShahError>;
#[derive(Debug)]
pub struct EntityInspector<T: EntityItem, S> {
    state: RefCell<S>,
    inspector: EntityInspectorFn<T, S>,
    _t: PhantomData<T>,
}

impl<T: EntityItem, S> EntityInspector<T, S> {
    pub fn new(state: S, inspector: EntityInspectorFn<T, S>) -> Self {
        Self { state: RefCell::new(state), inspector, _t: PhantomData::<T> }
    }

    fn call(&self, item: &T) -> Result<(), ShahError> {
        (self.inspector)(self.state.borrow_mut(), item)
    }
}

#[derive(Debug)]
pub struct EntityDb<
    T: EntityItem + EntityKochFrom<O, S>,
    O: EntityItem = T,
    S = (),
    Is = (),
> {
    file: File,
    pub live: GeneId,
    pub dead_list: DeadList<GeneId, BLOCK_SIZE>,
    revision: u16,
    name: String,
    koch: Option<EntityKoch<T, O, S>>,
    koch_prog: EntityKochProg,
    setup_prog: SetupProg,
    tasks: TaskList<2, Task<Self>>,
    ls: String,
    inspector: Option<EntityInspector<T, Is>>, // inspector: Option<fn(&mut Self, &T)>,
                                               // inspector: Option<fn(RefMut<Is>, &T)>,
                                               // inspector_state: Option<RefCell<Is>>,
}

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub fn new(path: &str, revision: u16) -> Result<Self, ShahError> {
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
            .open(path.join(format!("{name}.{revision}.shah")))?;

        let mut db = Self {
            live: GeneId(0),
            dead_list: DeadList::<GeneId, BLOCK_SIZE>::new(),
            file,
            revision,
            name: name.to_string(),
            koch: None,
            koch_prog: EntityKochProg::default(),
            setup_prog: SetupProg::default(),
            tasks: TaskList::new([Self::work_koch, Self::work_setup_task]),
            ls: format!("<EntityDb {name}.{revision} />"),
            inspector: None,
        };

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        self.init_head()?;
        self.koch_prog_get()?;

        self.live = GeneId(0);
        self.dead_list.clear();

        let file_size = self.file_size()?;
        if file_size < ENTITY_META {
            return Err(DbError::BadInit)?;
        }

        if file_size < ENTITY_META + T::N {
            self.file.write_all_at(T::default().as_binary(), ENTITY_META)?;
            return Ok(());
        }

        if file_size == ENTITY_META + T::N {
            return Ok(());
        }

        self.live = GeneId(((file_size - ENTITY_META) / T::N) - 1);

        self.setup_prog.prog = GeneId(1);
        self.setup_prog.total = self.live + 1;
        log::info!("{} init::setup_task {:?}", self.ls, self.setup_prog);

        Ok(())
    }

    fn init_head(&mut self) -> Result<(), ShahError> {
        let mut head = EntityHead::default();
        if let Err(e) = self.file.read_exact_at(head.as_binary_mut(), 0) {
            if e.kind() != ErrorKind::UnexpectedEof {
                return Err(e)?;
            }

            head.db_head.init(
                ENTITY_MAGIC,
                self.revision,
                &self.name,
                ENTITY_VERSION,
            );

            head.item_size = T::N;

            let svec = T::shah_schema().encode();
            head.schema[0..svec.len()].clone_from_slice(&svec);

            self.file.write_all_at(head.as_binary(), 0)?;

            return Ok(());
        }

        head.check::<T>(self.revision, &self.ls)?;

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

    pub fn set_koch(
        &mut self, koch: EntityKoch<T, O, S>,
    ) -> Result<(), ShahError> {
        self.koch_prog.total = koch.total;

        if !self.koch_prog.ended() {
            self.setup_prog.total = self.koch_prog.prog;
            self.setup_prog.prog = GeneId(1);
        }

        if self.live < koch.total {
            self.live = koch.total;
            utils::falloc(&self.file, ENTITY_META, (koch.total * T::N).0)?;
        }

        self.koch = Some(koch);

        Ok(())
    }

    pub fn set_inspector(&mut self, inspector: EntityInspector<T, Is>) {
        self.inspector = Some(inspector);
    }

    pub(crate) fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub(crate) fn total(&mut self) -> Result<GeneId, ShahError> {
        let file_size = self.file_size()?;
        if file_size < ENTITY_META {
            log::warn!("{} total file_size is less than ENTITY_META", self.ls);
            return Ok(GeneId(0));
        }
        if file_size < ENTITY_META + T::N {
            log::warn!(
                "{} total file_size is less than ENTITY_META + T::N",
                self.ls
            );
            return Ok(GeneId(0));
        }

        Ok(GeneId((file_size - ENTITY_META) / T::N - 1))
    }

    fn inspection(&mut self, entity: &T) {
        log::debug!("\x1b[36minspecting\x1b[m: {:?}", entity.gene());
        if !entity.is_alive() {
            let gene = entity.gene();
            log::debug!("{} inspector dead entity: {:?}", self.ls, gene.id);
            self.add_dead(gene);
        }

        if let Some(ei) = &self.inspector {
            if let Err(e) = ei.call(entity) {
                log::error!("inspection failed: {e:#?}");
            }
        }
    }

    fn work_koch(&mut self) -> Result<Performed, ShahError> {
        if self.koch.is_none() || self.koch_prog.ended() {
            return Ok(Performed(false));
        }

        let mut current = T::default();
        let mut performed = false;
        for _ in 0..10 {
            let Some(id) = self.koch_prog.next() else { break };
            let Some(koch) = self.koch.as_mut() else { break };

            let old = match koch.get_id(id) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("{} koch.get_id({id:?}): {e:?}", self.ls);
                    e.not_found_ok()?;
                    self.koch_prog.end();
                    break;
                }
            };
            performed = true;

            if self.read_at(&mut current, id).is_ok()
                && old.growth() <= current.growth()
            {
                // if we already did koch and updated the item do not koch again
                continue;
            }

            self.write_buf_at(&old, id)?;
            self.inspection(&old);
            log::debug!("koched: {:?}", old.gene());
        }

        if performed {
            self.koch_prog_set()?;
        }

        Ok(Performed(performed))
    }

    fn work_setup_task(&mut self) -> Result<Performed, ShahError> {
        if self.dead_list.is_full() || self.setup_prog.ended() {
            return Ok(Performed(false));
        }

        let mut entity = T::default();
        let mut performed = false;
        for _ in 0..10 {
            let Some(id) = self.setup_prog.next() else { break };
            performed = true;
            if let Err(e) = self.read_at(&mut entity, id) {
                e.not_found_ok()?;
                self.setup_prog.end();
                log::warn!(
                    "{} work_setup_task read_at not found {id:?}",
                    self.ls
                );
                break;
            }

            self.inspection(&entity);
        }

        Ok(Performed(performed))
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

    fn id_to_pos(id: GeneId) -> u64 {
        ENTITY_META + (id * T::N).0
    }

    pub(crate) fn seek_id(&mut self, id: GeneId) -> Result<(), ShahError> {
        self.file.seek(SeekFrom::Start(Self::id_to_pos(id)))?;
        Ok(())
    }

    pub(crate) fn write_buf_at<B: Binary>(
        &self, buf: &B, id: GeneId,
    ) -> Result<(), ShahError> {
        let pos = Self::id_to_pos(id);
        self.file.write_all_at(buf.as_binary(), pos)?;
        Ok(())
    }

    pub(crate) fn read_buf_at<B: Binary>(
        &self, buf: &mut B, id: GeneId,
    ) -> Result<(), ShahError> {
        let pos = Self::id_to_pos(id);
        match self.file.read_exact_at(buf.as_binary_mut(), pos) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => Err(NotFound::OutOfBounds)?,
                _ => Err(e)?,
            },
        }
    }

    pub(crate) fn read_at(
        &self, entity: &mut T, id: GeneId,
    ) -> Result<(), ShahError> {
        self.read_buf_at(entity, id)
    }

    pub(crate) fn del_unchecked(
        &mut self, entity: &mut T,
    ) -> Result<(), ShahError> {
        entity.set_alive(false);
        self.set_unchecked(entity)?;
        self.add_dead(entity.gene());
        Ok(())
    }

    pub(crate) fn set_unchecked(
        &mut self, entity: &mut T,
    ) -> Result<(), ShahError> {
        entity.growth_mut().add_assign(1);
        self.write_buf_at(entity, entity.gene().id)?;
        Ok(())
    }

    pub(crate) fn new_gene_id(&mut self) -> Result<GeneId, ShahError> {
        let pos = self.file.seek(SeekFrom::End(0))?;
        if pos < ENTITY_META + T::N {
            return Ok(GeneId(1));
        }

        let db_pos = pos - ENTITY_META;
        let (id, offset) = (db_pos / T::N, db_pos % T::N);
        if offset != 0 {
            log::warn!(
                "{} id: {id} | new-gene-id bad offset: {offset}",
                self.ls
            );
        }

        Ok(GeneId(id))
    }

    pub(crate) fn new_gene(&mut self) -> Result<Gene, ShahError> {
        let mut gene = Gene { id: self.take_dead_id(), ..Default::default() };
        utils::getrandom(&mut gene.pepper);
        gene.server = 0;
        gene.iter = 0;

        if gene.id != 0 {
            let mut old = T::default();
            if self.read_at(&mut old, gene.id).is_ok() {
                let og = old.gene();
                if og.iter < ITER_EXHAUSTION {
                    gene.iter = og.iter + 1;
                    return Ok(gene);
                }
            }
        }

        gene.id = self.new_gene_id()?;

        Ok(gene)
    }

    pub(crate) fn take_dead_id(&mut self) -> GeneId {
        self.dead_list.pop(|_| true).unwrap_or_default()
    }

    pub(crate) fn add_dead(&mut self, gene: &Gene) {
        if gene.id == 0 {
            return;
        }

        if self.live.0 > 0 {
            self.live -= 1;
        }

        if gene.iter >= ITER_EXHAUSTION {
            return;
        }

        self.dead_list.push(gene.id);
    }
}

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub fn get(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        gene.validate()?;

        self.read_at(entity, gene.id)?;

        let egene = entity.gene();
        if egene.id == 0 {
            if let Some(koch) = self.koch.as_mut() {
                let mut oldie = koch.get(gene)?;
                self.set_unchecked(&mut oldie)?;
                if !oldie.is_alive() {
                    self.add_dead(oldie.gene());
                    return Err(NotFound::EntityNotAlive)?;
                }
                entity.clone_from(&oldie);
                oldie.gene().check(gene)?;
                return Ok(());
            }

            return Err(SystemError::GeneIdMismatch)?;
        }
        egene.check(gene)?;

        if !entity.is_alive() {
            return Err(NotFound::EntityNotAlive)?;
        }

        Ok(())
    }

    pub fn add(&mut self, entity: &mut T) -> Result<(), ShahError> {
        entity.set_alive(true);
        let gene = entity.gene_mut();
        if gene.is_some() {
            log::warn!("{} entity gene is not cleared: {gene:?}", self.ls);
        }
        gene.clone_from(&self.new_gene()?);

        *entity.growth_mut() = 0;
        self.set_unchecked(entity)?;
        self.live += 1;

        Ok(())
    }

    pub fn count(&mut self) -> Result<EntityCount, ShahError> {
        Ok(EntityCount { total: self.total()?, alive: self.live })
    }

    pub fn set(&mut self, entity: &mut T) -> Result<(), ShahError> {
        if !entity.is_alive() {
            return Err(NotFound::DeadSet)?;
        }

        let mut old_entity = T::default();
        self.get(entity.gene(), &mut old_entity)?;
        // let growth = old_entity.growth();
        // let gene = old_entity.gene().clone();
        // old_entity.clone_from(&entity);
        // *old_entity.growth_mut() = growth;
        // old_entity.gene_mut().clone_from(&gene);

        *entity.growth_mut() = old_entity.growth();
        self.set_unchecked(entity)
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        // first make sure that the entity is alive and exists
        self.get(gene, entity)?;
        // then delete unchecked
        self.del_unchecked(entity)
    }

    pub fn list(
        &mut self, page: GeneId, result: &mut [T; PAGE_SIZE],
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
