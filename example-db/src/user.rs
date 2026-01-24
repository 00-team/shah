use crate::models::ExampleError;
use crate::models::State;
use shah::db::entity::EntityDb;
use shah::models::{Gene, GeneId};
use shah::{Entity, ShahError, ShahSchema};
use shah::{ErrorCode, PAGE_SIZE};

pub use db::User;

pub(crate) mod db {
    use shah::{db::entity::EntityFlags, models::ShahString};

    use super::*;

    #[shah::model]
    #[derive(Debug, PartialEq, ShahSchema)]
    pub struct SessionInfo {
        pub client: u8,
        pub os: u8,
        pub browser: u8,
        pub device: u8,
        pub client_version: u16,
        pub os_version: u16,
        pub browser_version: u16,
        _pad: [u8; 2],
    }

    #[shah::model]
    #[derive(Debug, PartialEq, ShahSchema)]
    pub struct Session {
        ip: [u8; 4],
        info: SessionInfo,
        timestamp: u64,
        token: [u8; 64],
    }

    #[shah::flags(inner = u8, serde = false)]
    pub struct UserFlags {
        pub is_banned: bool,
    }

    #[derive(ShahSchema)]
    #[shah::model]
    #[derive(Entity, Debug, PartialEq)]
    pub struct User {
        // pub flags: u64,
        pub gene: Gene,
        pub agent: Gene,
        pub review: Gene,
        pub photo: Gene,
        pub reviews: [u64; 3],
        phone: ShahString<12>,
        pub cc: u16,
        pub entity_flags: EntityFlags,
        flags: UserFlags,
        pub name: ShahString<48>,
        pub sessions: [Session; 3],
        growth: u64,
    }

    impl User {
        #[allow(dead_code)]
        pub fn set_phone(&mut self, phone: &str) -> Result<(), ExampleError> {
            if phone.len() != 11 || !phone.starts_with("09") {
                return Err(ExampleError::BadPhone);
            }
            if phone.chars().any(|c| !c.is_ascii_digit()) {
                return Err(ExampleError::BadPhone);
            }

            self.phone.set(phone);

            Ok(())
        }
    }

    pub type UserDb = EntityDb<User>;

    #[allow(dead_code)]
    pub(crate) fn init() -> Result<UserDb, ShahError> {
        UserDb::new("user", 1)
    }
}

#[shah::api(scope = 0, error = crate::models::ExampleError)]
mod uapi {
    use super::*;

    pub(super) fn user_add(
        state: &mut State, (inp,): (&User,), (out,): (&mut User,),
    ) -> Result<(), ErrorCode> {
        out.clone_from(inp);
        out.gene.id = GeneId(0);
        state.users.add(out)?;
        Ok(())
    }

    pub(crate) fn user_get(
        state: &mut State, (gene,): (&Gene,), (user,): (&mut User,),
    ) -> Result<(), ErrorCode> {
        log::debug!("in user::user_get ");
        log::debug!("inp: {:?}", gene);
        log::debug!("out: {:?}", user);

        state.users.get(gene, user)?;

        log::debug!("out: {:?}", user);

        Ok(())
    }

    pub(crate) fn user_list(
        state: &mut State, (page,): (&GeneId,),
        (users,): (&mut [User; PAGE_SIZE],),
    ) -> Result<usize, ErrorCode> {
        let count = state.users.list(*page, users)?;
        Ok(count * <User as shah::models::Binary>::S)
    }
}
