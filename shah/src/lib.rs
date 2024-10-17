mod binary;
pub mod db;
pub mod error;
pub mod models;
pub mod server;
mod taker;
pub(crate) mod utils;

pub use crate::binary::*;
pub use crate::error::{ClientError, ErrorCode};
pub use models::*;
pub use taker::Taker;

pub use shah_macros::{api, enum_code, model, Entity};

#[allow(unused_extern_crates)]
extern crate self as shah;

type ApiCaller<T> = fn(&mut T, &[u8], &mut [u8]) -> Result<usize, ErrorCode>;
#[derive(Debug)]
pub struct Api<T> {
    pub name: &'static str,
    pub input_size: usize,
    pub caller: ApiCaller<T>,
}
