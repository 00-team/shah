use shah::{db::snake::SnakeDb, error::IsNotFound, ErrorCode};

use crate::{note::db::NoteDb, phone::db::PhoneDb, user::db::UserDb};

#[derive(Debug)]
pub struct State {
    pub users: UserDb,
    pub phone: PhoneDb,
    pub detail: SnakeDb,
    pub notes: NoteDb,
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
