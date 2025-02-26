use shah::{
    db::{entity::EntityKoch, snake::SnakeDb},
    error::{IsNotFound, ShahError},
    models::{Performed, ShahState, Task, TaskList},
    ErrorCode,
};

use crate::{
    extra::db::ExtraDb,
    note::db::NoteDb,
    phone::db::PhoneDb,
    user,
};

pub struct State {
    pub users: user::db::UserDb,
    pub phone: PhoneDb,
    pub detail: SnakeDb,
    pub notes: NoteDb,
    pub extra: ExtraDb,
    tasks: TaskList<3, Task<Self>>,
}

macro_rules! work_fn {
    ($name:ident, $fn_name:ident) => {
        fn $fn_name(&mut self) -> Result<Performed, ShahError> {
            self.$name.work()
        }
    };
}

impl State {
    pub fn new(
        users: user::db::UserDb, phone: PhoneDb, detail: SnakeDb,
        notes: NoteDb, extra: ExtraDb,
    ) -> Result<Self, ShahError> {
        Ok(Self {
            users,
            phone,
            detail,
            notes,
            extra,
            tasks: TaskList::new([
                Self::work_users,
                Self::work_notes,
                Self::work_detail,
            ]),
        }
        .init()?)
    }
    pub fn init(mut self) -> Result<Self, ShahError> {
        let mig = EntityKoch::new(user::db::old_init()?, ());
        self.users.set_koch(mig)?;
        // let x = RefCell::new(self);
        // let mut s = x.borrow_mut();
        // let ng = s.users.new_gene();

        Ok(self)
        // Ok(())
    }

    work_fn!(users, work_users);
    work_fn!(notes, work_notes);
    work_fn!(detail, work_detail);
}

impl ShahState for State {
    fn work(&mut self) -> Result<Performed, ShahError> {
        self.tasks.start();
        while let Some(task) = self.tasks.next() {
            if task(self)?.0 {
                return Ok(Performed(true));
            }
        }
        Ok(Performed(false))
    }
}

// pub type ExampleApi = shah::Api<State<'static>>;

#[shah::enum_int(ty = u16)]
#[derive(Debug, Default, Clone, Copy)]
/// example errors
pub enum ExampleError {
    #[default]
    Unknown = 0,
    UserNotFound,
    BadPhone,
    BadStr,
}

impl IsNotFound for ExampleError {
    fn is_not_found(&self) -> bool {
        matches!(self, Self::UserNotFound | Self::BadPhone)
        // match self {
        //     Self::UserNotFound | Self::BadPhone => true,
        //     _ => false,
        // }
    }
}

impl From<ExampleError> for ErrorCode {
    fn from(value: ExampleError) -> Self {
        Self::user(value as u16)
    }
}
