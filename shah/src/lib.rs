mod binary;
pub mod db;
pub mod dead_list;
pub mod state;
pub mod error;
pub mod models;
pub mod perms;
pub mod schema;
pub mod server;
mod taker;
pub(crate) mod utils;

pub use crate::binary::*;
pub use crate::dead_list::DeadList;
pub use crate::error::{ClientError, ErrorCode};
pub use models::*;
pub use taker::Taker;

pub use shah_macros::{
    api, enum_int, model, perms, routes, Command, Duck, Entity, EnumCode,
    ShahSchema,
};

pub const PAGE_SIZE: usize = 0x20;
pub const BLOCK_SIZE: usize = 0x1000;
pub const ITER_EXHAUSTION: u8 = 0xfa;

#[allow(unused_extern_crates)]
extern crate self as shah;

type ApiCaller<T> = fn(&mut T, &[u8], &mut [u8]) -> Result<usize, ErrorCode>;

#[derive(Debug)]
pub struct Api<T> {
    pub name: &'static str,
    pub input_size: usize,
    pub caller: ApiCaller<T>,
}

#[derive(Debug)]
pub struct Scope<T: 'static> {
    pub routes: &'static [Api<T>],
    pub name: &'static str,
    pub scope: usize,
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

pub trait AsUtf8Str {
    fn as_utf8_str(&self) -> &str;
}

impl AsUtf8Str for [u8] {
    fn as_utf8_str(&self) -> &str {
        match core::str::from_utf8(self) {
            Ok(v) => v,
            Err(e) => core::str::from_utf8(&self[..e.valid_up_to()])
                .unwrap_or_default(),
        }
    }
}
