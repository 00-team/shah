mod db {
    use shah::entity::{Entity, EntityDb};
    use shah::Binary;
    use shah::Gene;

    #[shah::model]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct User {
        pub gene: Gene,
        pub flags: u64,
        pub name: [u8; 12],
        pub age: u32,
    }

    impl Entity for User {
        fn gene(&self) -> &Gene {
            &self.gene
        }
        fn flags(&self) -> &u8 {
            &self.flags.as_binary()[0]
        }

        fn gene_mut(&mut self) -> &mut Gene {
            &mut self.gene
        }
        fn flags_mut(&mut self) -> &mut u8 {
            &mut self.flags.as_binary_mut()[0]
        }
    }

    pub(crate) fn setup() -> EntityDb<User> {
        EntityDb::<User>::new("user").expect("user db setup")
    }
}

#[shah::api(ExampleApi)]
mod api {
    use super::User;
    use crate::models::{ExampleApi, State};
    use shah::{ErrorCode, Gene};

    pub(crate) fn user_get(
        state: &mut State, (gene, ): (&Gene, ), (user, ): (&mut User,),
    ) -> Result<(), ErrorCode> {

        state.users.get(gene, user)?;

        log::debug!("in user::user_get ");
        log::debug!("state.users.file: {:?}", state.users.file);
        log::debug!("inp: {:?}", gene);
        log::debug!("out: {:?}", user);

        Ok(())
    }
}

pub use api::*;
pub use db::*;
