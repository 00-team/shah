pub mod db {
    use shah::entity::EntityDb;
    use shah::Gene;
    use shah::{Binary, Entity};

    #[shah::model]
    #[derive(Entity, Debug, PartialEq, Clone, Copy)]
    pub struct User {
        pub gene: Gene,
        pub flags: u8,
        pub _pad: [u8; 7],
        #[str]
        pub name: [u8; 12],
        pub age: u32,
    }

    pub(crate) fn setup() -> EntityDb<User> {
        EntityDb::<User>::new("user").expect("user db setup")
    }
}

#[shah::api(scope = 0, api = crate::models::ExampleApi, error = crate::models::ExampleError)]
mod api {
    use super::db::User;
    use crate::models::State;
    use shah::{ErrorCode, Gene, GeneId};

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
        state: &mut State, inp: (&GeneId,), out: (&mut [User; 32],),
    ) -> Result<usize, ErrorCode> {
        Ok(0)
    }

    pub(crate) fn user_test(
        state: &mut State, inp: (&u8, &u16, &[u8; 4096]), _: (),
    ) -> Result<(), ErrorCode> {
        log::debug!("user_test: inp: {inp:?}");

        Ok(())
    }
}
