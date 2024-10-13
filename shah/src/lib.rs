mod binary;
pub mod entity;
pub mod error;
pub mod models;
pub mod server;
mod taker;
pub(crate) mod utils;

pub use crate::binary::{Binary, FromBytes};
pub use crate::error::{ClientError, ErrorCode};
pub use models::*;
pub use taker::Taker;

pub use shah_macros::{api, enum_code, model};

#[allow(unused_extern_crates)]
extern crate self as shah;

#[derive(Debug)]
pub struct Api<T> {
    pub name: &'static str,
    pub input_min: usize,
    pub input_max: usize,
    pub caller: fn(&mut T, &[u8], &mut [u8]) -> Result<usize, ErrorCode>,
}
