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

pub use shah_macros::{api, enum_code, model, Command, Entity};

pub const PAGE_SIZE: usize = 32;

#[allow(unused_extern_crates)]
extern crate self as shah;

type ApiCaller<T> = fn(&mut T, &[u8], &mut [u8]) -> Result<usize, ErrorCode>;
#[derive(Debug)]
pub struct Api<T> {
    pub name: &'static str,
    pub input_size: usize,
    pub caller: ApiCaller<T>,
}

pub trait Command {
    fn parse(args: std::env::Args) -> Self;
    fn help() -> String;
}

pub fn command<T: Command + Default>() -> T {
    let mut args = std::env::args();
    loop {
        let Some(arg) = args.next() else { break T::default() };
        if arg == "-c" {
            break T::parse(args);
        }
    }
}
