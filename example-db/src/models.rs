use shah::{
    db::entity::EntityKoch,
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

unsafe fn extend_lifetime<T>(r: &mut T) -> &'static mut T {
    // one liner
    // &mut *(r as *mut T)

    // Convert the mutable reference to a raw pointer
    let raw_ptr: *mut T = r;

    // Convert the raw pointer back to a mutable reference with 'static lifetime
    &mut *raw_ptr
}

impl State {
    pub fn init(mut self) -> Result<Self, ShahError> {
        let mig = EntityKoch::new(user::db::old_init()?, unsafe {
            extend_lifetime(&mut self)
        });
        self.users.set_koch(mig);
        // let x = RefCell::new(self);
        // let mut s = x.borrow_mut();
        // let ng = s.users.new_gene();

        Ok(self)
        // Ok(())
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
