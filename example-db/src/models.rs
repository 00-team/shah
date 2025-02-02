use std::io::Seek;

use shah::{
    db::snake::SnakeDb,
    error::IsNotFound,
    state::{ShahState, Task},
    ErrorCode,
};

use crate::{note::db::NoteDb, phone::db::PhoneDb, user::db::UserDb};

#[derive(Debug)]
pub struct State {
    pub users: UserDb,
    pub phone: PhoneDb,
    pub detail: SnakeDb,
    pub notes: NoteDb,
}

struct UserDbMigTask<'a> {
    state: &'a mut State,
    total: u64,
    progress: u64,
}

impl<'a> Task for UserDbMigTask<'a> {
    fn work(&mut self) {
        for id in self.progress..(self.total - self.progress).min(10) {
            self.state.users.file.seek_relative(id as i64);
            self.progress += 1;
        }
    }
}

impl<'a> ShahState<'a> for State {
    fn tasks(&'a mut self) -> &'a [impl Task] {
        let usbm = UserDbMigTask { state: self, total: 11, progress: 0 };
        // self.users.set_migration();
        &[usbm]
    }
}

pub type ExampleApi = shah::Api<State>;

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
        match self {
            Self::UserNotFound | Self::BadPhone => true,
            _ => false,
        }
    }
}

impl From<ExampleError> for ErrorCode {
    fn from(value: ExampleError) -> Self {
        Self::user(value as u16)
    }
}
