mod server;
mod taker;

pub mod db;
pub mod error;
pub mod models;

pub(crate) mod utils;

pub use error::*;
pub use server::run;
pub use taker::*;

pub use shah_macros::*;

pub const PAGE_SIZE: usize = 32;
pub const BLOCK_SIZE: usize = 4096;
pub const ITER_EXHAUSTION: u8 = 250;

#[allow(unused_extern_crates)]
extern crate self as shah;

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
