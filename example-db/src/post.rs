mod db {
    use shah::entity::{Entity, EntityDb};
    use shah::Binary;
    use shah::Gene;

    #[shah::model]
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Post {
        pub gene: Gene,
        pub flags: u64,
        pub title: [u8; 16],
        pub timestamp: u64,
    }

    impl Entity for Post {
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

    pub fn setup() -> EntityDb<Post> {
        EntityDb::<Post>::new("post").expect("post db setup")
    }
}

#[shah::api(crate::models::ExampleApi)]
mod api {
    use super::db::Post;
    use crate::models::State;
    use shah::{ErrorCode, Gene};

    pub fn post_get(
        state: &mut State, inp: (&Gene,), out: (&mut Post,),
    ) -> Result<(), ErrorCode> {
        log::debug!("in post::post_get ");
        log::debug!("state: {state:?}");
        log::debug!("inp: {inp:?}");
        log::debug!("out: {out:?}");

        Ok(())
    }
    // pub fn post_get_api(
    //         state: &State, inp: &[u8], out: &mut [u8],
    //     ) -> Result<(), ErrorCode> {
    //         let gene = Gene::from_binary(&inp[0..Gene::S]);
    //         let post = Post::from_binary_mut(&mut out[0..Post::S]);
    //         post_get(state, (gene,), (post,))
    //     }

    pub fn post_add(
        state: &mut State, inp: (&Post,), out: (&mut Post,),
    ) -> Result<(), ErrorCode> {
        Ok(())
    }

    // pub fn post_add_api(
    //     state: &State, inp: &[u8], out: &mut [u8],
    // ) -> Result<(), ErrorCode> {
    //     let inp_post = Post::from_binary(&inp[0..Post::S]);
    //     let out_post = Post::from_binary_mut(&mut out[0..Post::S]);
    //     post_add(state, (inp_post,), (out_post,))
    // }
    //
    // pub const ROUTES: [ExampleApi; 2] = [
    //     ExampleApi {
    //         input_size: Gene::S,
    //         output_size: Post::S,
    //         caller: post_get_api,
    //     },
    //     ExampleApi {
    //         input_size: Post::S,
    //         output_size: Post::S,
    //         caller: post_add_api,
    //     },
    // ];
}

pub(crate) use api::*;
pub use db::*;
