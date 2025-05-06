use std::cell::{RefCell, RefMut};
use std::marker::PhantomData;
use std::ops::AddAssign;
use std::path::Path;
use std::{
    fmt::Debug,
    fs::File,
    io::{ErrorKind, Seek, SeekFrom},
    os::unix::fs::FileExt,
};

use super::*;
use crate::config::ShahConfig;
use crate::models::*;
use crate::*;

mod dead;
mod init;
mod options;
mod api;
mod util;
mod work;

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
    live: GeneId,
    dead_list: DeadList<GeneId, BLOCK_SIZE>,
    revision: u16,
    name: String,
    koch: Option<EntityKoch<T, O, S>>,
    koch_prog: EntityKochProg,
    setup_prog: SetupProg,
    tasks: TaskList<2, Task<Self>>,
    ls: String,
    inspector: Option<EntityInspector<T, Is>>,
    work_iter: usize,
}
