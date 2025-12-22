use std::cell::{RefCell, RefMut};
use std::fs::File;
use std::io::{ErrorKind, Seek, SeekFrom};
use std::marker::PhantomData;
use std::os::unix::fs::FileExt;

use super::{ENTITY_META, EntityHead, EntityItem, id_iter};
use crate::config::ShahConfig;
use crate::models::{Binary, Gene, GeneId};
use crate::{DbError, NotFound, ShahError, SystemError, utils};

// =========== EntityKochFrom trait ===========

pub trait EntityKochFrom<Old: EntityItem, State = ()>: Sized {
    fn entity_koch_from(
        old: Old, state: RefMut<State>,
    ) -> Result<Self, ShahError>;
}

impl<T: EntityItem, S> EntityKochFrom<T, S> for T {
    fn entity_koch_from(old: T, _: RefMut<S>) -> Result<Self, ShahError> {
        Ok(old)
    }
}

// =========== end of EntityKochFrom trait ===========

// =========== EntityKochProg struct ===========

#[crate::model]
#[derive(Debug)]
pub struct EntityKochProg {
    pub total: GeneId,
    pub prog: GeneId,
}

id_iter!(EntityKochProg);

// =========== end of EntityKochProg struct ===========

// =========== EntityKoch struct ===========

#[derive(Debug)]
pub struct EntityKoch<New, Old: EntityItem, State>
where
    New: EntityItem + EntityKochFrom<Old, State>,
{
    pub from: EntityKochDb<Old>,
    pub state: RefCell<State>,
    pub total: GeneId,
    // pub prog: u64,
    _new: PhantomData<New>,
}

impl<New, Old, State> EntityKoch<New, Old, State>
where
    New: EntityItem + EntityKochFrom<Old, State>,
    Old: EntityItem,
{
    pub fn new(
        from: Result<EntityKochDb<Old>, ShahError>, state: State,
    ) -> Option<Self> {
        let from = match from {
            Ok(db) => db,
            Err(e) => {
                log::error!("failed to init koch db: {e:#?}");
                return None;
            }
        };

        Some(Self {
            // prog: 0,
            total: from.total,
            from,
            state: RefCell::new(state),
            _new: PhantomData::<New>,
        })
    }

    pub fn get_id(&self, gene_id: GeneId) -> Result<New, ShahError> {
        if gene_id == 0 {
            return Ok(New::default());
        }

        let mut old = Old::default();
        self.from.get_id(gene_id, &mut old)?;
        New::entity_koch_from(old, self.state.borrow_mut())
    }

    pub fn get(&self, gene: &Gene) -> Result<New, ShahError> {
        if gene.id == 0 {
            return Ok(New::default());
        }

        let mut old = Old::default();
        self.from.get(gene, &mut old)?;
        New::entity_koch_from(old, self.state.borrow_mut())
    }
}

// =========== end of EntityKoch struct ===========

// =========== EntityKochDb struct ===========

#[derive(Debug)]
pub struct EntityKochDb<T: EntityItem> {
    file: File,
    revision: u16,
    total: GeneId,
    ls: String,
    _e: PhantomData<T>,
}

impl<T: EntityItem> EntityKochDb<T> {
    pub fn new(path: &str, revision: u16) -> Result<Self, ShahError> {
        let conf = ShahConfig::get();
        let path = conf.data_dir.join(path);
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        let open_path = path.join(format!("{name}.{revision}.shah"));
        log::debug!("opening: {open_path:?} for koching");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .truncate(false)
            .open(open_path)?;

        let mut db = Self {
            file,
            revision,
            total: GeneId(0),
            ls: format!("<EntityKochDb {name}.{revision}>"),
            _e: PhantomData::<T>,
        };

        db.init()?;

        Ok(db)
    }

    pub fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    fn init(&mut self) -> Result<(), ShahError> {
        let file_size = self.file_size()?;
        if file_size < ENTITY_META + T::N {
            log::error!("{} db content is not valid", self.ls);
            return Err(DbError::InvalidDbContent)?;
        }

        self.check_head()?;

        self.total = GeneId((file_size - ENTITY_META) / T::N);

        Ok(())
    }

    fn check_head(&self) -> Result<(), ShahError> {
        let mut head = EntityHead::default();
        self.file.read_exact_at(head.as_binary_mut(), 0)?;

        head.check::<T>(self.revision, &self.ls)?;

        Ok(())
    }

    // pub fn seek_id(&mut self, id: GeneId) -> Result<(), ShahError> {
    //     let pos = ENTITY_META + id * T::N;
    //     self.file.seek(SeekFrom::Start(pos))?;
    //     Ok(())
    // }

    fn id_to_pos(id: GeneId) -> u64 {
        ENTITY_META + (id * T::N).0
    }

    pub fn read_buf_at<B: Binary>(
        &self, buf: &mut B, id: GeneId,
    ) -> Result<(), ShahError> {
        let pos = Self::id_to_pos(id);
        match self.file.read_exact_at(buf.as_binary_mut(), pos) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => Err(NotFound::OutOfBounds)?,
                _ => {
                    log::error!("{} read_buf_at: {e:?}", self.ls);
                    Err(e)?
                }
            },
        }
    }

    pub fn read_at(&self, entity: &mut T, id: GeneId) -> Result<(), ShahError> {
        self.read_buf_at(entity, id)
    }

    pub fn get_id(
        &self, gene_id: GeneId, entity: &mut T,
    ) -> Result<(), ShahError> {
        self.read_at(entity, gene_id)?;

        if entity.gene().is_none() {
            return Err(NotFound::EmptyItem)?;
        }

        if gene_id != entity.gene().id {
            log::error!("{} get_id: gene id mismatch", self.ls);
            return Err(SystemError::GeneIdMismatch)?;
        }

        Ok(())
    }

    pub fn get(&self, gene: &Gene, entity: &mut T) -> Result<(), ShahError> {
        gene.validate()?;
        self.read_at(entity, gene.id)?;
        gene.check(entity.gene(), &self.ls)?;
        Ok(())
    }
}

// =========== end of EntityKochDb struct ===========
