use shah::ErrorCode;

use crate::{phone::db::PhoneDb, user::db::UserDb};

#[derive(Debug)]
pub struct State {
    pub users: UserDb,
    pub phone: PhoneDb,
}

pub type ExampleApi = shah::Api<State>;

/// example errors
#[derive(Debug)]
#[repr(u16)]
pub enum ExampleError {
    UserNotFound,
    BadPhone,
}

impl From<ExampleError> for ErrorCode {
    fn from(value: ExampleError) -> Self {
        Self::user(value as u16)
    }
}

impl From<u16> for ExampleError {
    fn from(value: u16) -> Self {
        unsafe { core::mem::transmute(value) }
    }
}
