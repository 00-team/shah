pub const DETAIL_MAX: usize = 10_000;
pub const DETAIL_BUF: usize = 255;

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
    use shah::{
        db::snake::SnakeHead, AsUtf8Str, ClientError, ErrorCode, Gene, Taker,
        BLOCK_SIZE,
    };

    use super::{DETAIL_BUF, DETAIL_MAX};

    pub(crate) fn init(
        state: &mut State, (capacity,): (&u64,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.alloc(*capacity, head)?)
    }

    pub(crate) fn head(
        state: &mut State, (gene,): (&Gene,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.index.get(gene, head)?)
    }

    pub(crate) fn read(
        state: &mut State, (gene, offset): (&Gene, &u64),
        (head, buf): (&mut SnakeHead, &mut [u8; BLOCK_SIZE]),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.read(gene, head, *offset, buf)?)
    }

    pub(crate) fn write(
        state: &mut State,
        (gene, offset, data, len): (&Gene, &u64, &[u8; BLOCK_SIZE], &u64),
        (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.write(gene, head, *offset, &data[0..*len as usize])?)
    }

    pub(crate) fn free(
        state: &mut State, (gene,): (&Gene,), (head,): (&mut SnakeHead,),
    ) -> Result<(), ErrorCode> {
        Ok(state.detail.free(gene, head)?)
    }

    #[client]
    pub fn get(
        taker: &mut Taker, gene: &Gene,
    ) -> Result<String, ClientError<ExampleError>> {
        let (head, buf) = read(taker, gene, &0)?;
        let len = head.length.min(head.capacity);
        let len = if len == 0 { head.capacity } else { len } as usize;
        let mut v = Vec::with_capacity(len);

        if len > BLOCK_SIZE {
            v[0..BLOCK_SIZE].copy_from_slice(buf);
            for i in 1..=(len / BLOCK_SIZE) {
                let off = i * BLOCK_SIZE;
                let (_, buf) = read(taker, gene, &(off as u64))?;
                v[off..(off + BLOCK_SIZE).min(len)]
                    .copy_from_slice(&buf[..(len - off).min(BLOCK_SIZE)])
            }
        } else {
            v[0..len].copy_from_slice(&buf[0..len]);
        }

        Ok(v.as_utf8_str().to_string())
    }

    #[client]
    pub fn set(
        taker: &mut Taker, gene: &Option<Gene>, data: &str,
    ) -> Result<Gene, ClientError<ExampleError>> {
        let data = data.as_bytes();
        let len = data.len().min(DETAIL_MAX);
        let mut snake: Option<SnakeHead> = None;
        if let Some(old) = gene {
            let (old_head,) = head(taker, old)?;
            if old_head.capacity >= len as u64 {
                snake = Some(old_head.clone());
            } else {
                free(taker, old)?;
            }
        }
        if snake.is_none() {
            let capacity = (len + DETAIL_BUF).min(DETAIL_MAX) as u64;
            snake = Some(init(taker, &capacity)?.0.clone());
        }
        let snake = snake.unwrap();
        for i in 0..(len / BLOCK_SIZE) {
            let off = (i * BLOCK_SIZE);
            let mut write_buffer = [0u8; BLOCK_SIZE];
            if len < off + BLOCK_SIZE {
                let wlen = (off + BLOCK_SIZE) - len;
                write_buffer[0..wlen].copy_from_slice(&data[off..len]);
                write(
                    taker,
                    &snake.gene,
                    &(off as u64),
                    &write_buffer,
                    &(wlen as u64),
                )?;
            } else {
                write(
                    taker,
                    &snake.gene,
                    &(off as u64),
                    &data[off..off + BLOCK_SIZE].try_into().unwrap(),
                    &(BLOCK_SIZE as u64),
                )?;
            }
        }

        Ok(snake.gene)
    }
}
