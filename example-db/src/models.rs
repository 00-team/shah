use shah::{
    db::entity::EntityMigration,
    error::{IsNotFound, ShahError},
    ErrorCode,
};

use crate::{phone::db::PhoneDb, user};

#[derive(Debug)]
pub struct State {
    pub users: user::db::UserDb,
    pub phone: PhoneDb,
    // pub detail: SnakeDb,
    // pub notes: NoteDb,
}

impl State {
    pub fn init(mut self) -> Result<Self, ShahError> {
        self.users.set_migration(EntityMigration {
            from: user::db::old_init()?,
            state: (),
        });

        Ok(self)
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
