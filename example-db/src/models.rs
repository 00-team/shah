use shah::{entity::EntityDb, ErrorCode};

#[derive(Debug)]
pub struct State {
    pub users: EntityDb<crate::user::User>,
    pub posts: EntityDb<crate::post::Post>,
}

pub type ExampleApi = shah::Api<State>;

/// example errors
#[derive(Debug)]
#[repr(u16)]
pub enum ExampleError {
    UserNotFound,
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
