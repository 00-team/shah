use shah::{
    db::snake::SnakeDb,
    error::IsNotFound,
    state::{ShahState, Task},
    ErrorCode,
};

use crate::{note::db::NoteDb, phone::db::PhoneDb, user::db::UserDb};

#[derive(Debug)]
pub struct State<'a> {
    pub users: UserDb<'a>,
    pub phone: PhoneDb,
    pub detail: SnakeDb,
    pub notes: NoteDb,
}

struct UserDbMigTask<'a> {
    _state: &'a mut State<'a>,
    total: u64,
    progress: u64,
}

impl<'a> Task for UserDbMigTask<'a> {
    fn work(&mut self) {
        for id in self.progress..(self.total - self.progress).min(10) {
            log::info!("mig task: {id}");
            // self.state.users.file.seek_relative(id as i64);
            self.progress += 1;
        }
    }
}

impl<'a> ShahState<'a> for State<'a> {
    fn tasks(&'a mut self) -> Vec<impl Task> {
        let usbm = UserDbMigTask { _state: self, total: 11, progress: 0 };
        // self.users.set_migration();
        vec![usbm]
    }
}

pub type ExampleApi = shah::Api<State<'static>>;

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
