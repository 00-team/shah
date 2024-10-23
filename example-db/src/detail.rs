pub mod db {
    use shah::db::snake::SnakeDb;

    pub fn setup() -> SnakeDb {
        let db = SnakeDb::new("detail").expect("detail setup");
        db.setup().expect("detail setup")
    }
}

#[shah::api(scope = 3, error = ExampleError, api = ExampleApi)]
pub mod api {
    use crate::models::{ExampleApi, ExampleError, State};
    use shah::{db::snake::SnakeHead, ErrorCode};

    pub(crate) fn init(
        state: &mut State, (capacity,): (&u64,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        head.clone_from(&state.detail.alloc(*capacity)?);
        Ok(())
    }

    pub(crate) fn read(
        state: &mut State, (gene, offset ,): (&Gene, &u64,), (head, buf,): (&mut SnakeHead, &mut [u8; 4096],),
    ) -> Result<(), ErrorCode> {
        head.clone_from(state.detail.read(gene, *offset, buf));
        Ok(())
    }

pub(crate) fn init(
        state: &mut State, (capacity,): (&u64,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        head.clone_from(&state.detail.alloc(*capacity)?);
        Ok(())
    }

pub(crate) fn init(
        state: &mut State, (capacity,): (&u64,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        head.clone_from(&state.detail.alloc(*capacity)?);
        Ok(())
    }

}
