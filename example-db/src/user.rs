pub mod db {
    #![allow(dead_code)]

    use shah::db::entity::EntityDb;
    use shah::error::ShahError;
    use shah::Entity;
    use shah::Gene;
    use shah::ShahSchema;

    use crate::models::ExampleError;

    pub type UserDb = EntityDb<User>;

    #[shah::model]
    #[derive(Debug, PartialEq, Clone, Copy, ShahSchema)]
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
    #[derive(Debug, PartialEq, Clone, Copy, ShahSchema)]
    pub struct Session {
        ip: [u8; 4],
        info: SessionInfo,
        timestamp: u64,
        token: [u8; 64],
    }

    #[shah::model]
    #[derive(Entity, Debug, PartialEq, Clone, Copy, ShahSchema)]
    pub struct User {
        // pub flags: u64,
        pub gene: Gene,
        pub agent: Gene,
        pub review: Gene,
        pub photo: Gene,
        pub reviews: [u64; 3],
        #[str(set = false)]
        phone: [u8; 12],
        pub cc: u16,
        #[entity_flags]
        pub entity_flags: u8,
        #[flags(banned)]
        pub flags: u8,
        #[str]
        pub name: [u8; 48],
        pub sessions: [Session; 3],
    }

    impl User {
        pub fn set_phone(&mut self, phone: &str) -> Result<(), ExampleError> {
            if phone.len() != 11 || !phone.starts_with("09") {
                return Err(ExampleError::BadPhone);
            }
            if phone.chars().any(|c| !c.is_ascii_digit()) {
                return Err(ExampleError::BadPhone);
            }

            self.phone[..11].clone_from_slice(phone.as_bytes());

            Ok(())
        }
    }

    pub(crate) fn setup() -> Result<UserDb, ShahError> {
        UserDb::new("user")?.setup(|_, _| {})
    }
}

#[shah::api(scope = 0, api = crate::models::ExampleApi, error = crate::models::ExampleError)]
mod api {
    use super::db::User;
    use crate::models::State;
    use shah::{ErrorCode, Gene, GeneId, PAGE_SIZE};

    pub(crate) fn user_add(
        state: &mut State, (inp,): (&User,), (out,): (&mut User,),
    ) -> Result<(), ErrorCode> {
        out.clone_from(inp);
        out.gene.id = 0;
        state.users.add(out)?;
        Ok(())
    }

    pub(crate) fn user_get(
        state: &mut State, (gene,): (&Gene,), (user,): (&mut User,),
    ) -> Result<(), ErrorCode> {
        log::debug!("in user::user_get ");
        log::debug!("state.users.file: {:?}", state.users.file);
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
        Ok(count * <User as shah::Binary>::S)
    }
}
