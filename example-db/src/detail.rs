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
    use shah::{db::snake::SnakeHead, ClientError, ErrorCode, Gene, Taker};

    pub(crate) fn init(
        state: &mut State, (capacity,): (&u64,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.alloc(*capacity, head)?)
    }

    pub(crate) fn read(
        state: &mut State, (gene, offset): (&Gene, &u64),
        (head, buf): (&mut SnakeHead, &mut [u8; 4096]),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.read(gene, head, *offset, buf)?)
    }

    #[client]
    pub(crate) fn get(
        taker: &mut Taker, gene: &Gene, offset: u64,
    ) -> Result<String, ClientError<ExampleError>> {
        Ok(String::new())
    }

    // pub fn read() -> Result<String>

    pub(crate) fn write(
        state: &mut State,
        (gene, offset, data, length): (&Gene, &u64, &[u8; 4094], &u64),
        (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.write(
            gene,
            head,
            *offset,
            &data[0..*length as usize],
        )?)
    }

    pub(crate) fn free(
        state: &mut State, (gene,): (&Gene,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.free(gene, head)?)
    }
}
