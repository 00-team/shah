mod binary;
pub mod entity;
pub mod error;
pub mod models;
pub mod server;
pub(crate) mod utils;

pub use crate::binary::Binary;
pub use crate::error::{ErrorCode, ClientError};
pub use models::*;

pub use shah_macros::{api, enum_code, model};

#[allow(unused_extern_crates)]
extern crate self as shah;

#[derive(Debug)]
pub struct Api<T> {
    pub name: &'static str,
    pub input_size: usize,
    pub output_size: usize,
    pub caller: fn(&mut T, &[u8], &mut [u8]) -> Result<(), ErrorCode>,
}

pub trait Taker {
    fn take(&self, order: &[u8]) -> Result<&[u8], ErrorCode>;
}
