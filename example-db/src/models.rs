use shah::{entity::EntityDb, ErrorCode};

#[derive(Debug)]
pub struct State {
    pub users: EntityDb<crate::user::User>,
    pub posts: EntityDb<crate::post::Post>,
}

pub type ExampleApi = shah::Api<State>;

/// example errors
#[derive(Debug)]
#[shah::enum_code]
pub enum ExampleError {
    UserNotFound,
}

impl From<ExampleError> for ErrorCode {
    fn from(value: ExampleError) -> Self {
        Self::user(value)
    }
}
