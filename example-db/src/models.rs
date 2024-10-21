use std::str::Utf8Error;

use shah::{db::snake::SnakeDb, error::SystemError, ErrorCode};

use crate::{phone::db::PhoneDb, user::db::UserDb};

#[derive(Debug)]
pub struct State {
    pub users: UserDb,
    pub phone: PhoneDb,
    pub detail: SnakeDb,
}

pub type ExampleApi = shah::Api<State>;

/// example errors
#[derive(Debug)]
#[repr(u16)]
pub enum ExampleError {
    UserNotFound,
    BadPhone,
    BadStr,
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

pub enum MyErr {
    System(SystemError),
    User(ExampleError),
}

impl From<MyErr> for ErrorCode {
    fn from(value: MyErr) -> Self {
        match value {
            MyErr::User(e) => ErrorCode::user(e as u16),
            MyErr::System(e) => ErrorCode::system(e as u16),
        }
    }
}

impl From<Utf8Error> for MyErr {
    fn from(_: Utf8Error) -> Self {
        MyErr::User(ExampleError::BadStr)
    }
}

impl From<SystemError> for MyErr {
    fn from(value: SystemError) -> Self {
        MyErr::System(value)
    }
}
