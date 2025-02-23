mod server;
mod taker;

pub mod db;
pub mod error;
pub mod models;

pub(crate) mod utils;

pub use error::*;
pub use server::run;
pub use taker::*;
pub use utils::AsStatic;

pub use shah_macros::*;

pub const PAGE_SIZE: usize = 32;
pub const BLOCK_SIZE: usize = 4096;
pub const ITER_EXHAUSTION: u8 = 250;
pub const VERSION_MAJOR: u16 = utils::env_num(env!("CARGO_PKG_VERSION_MAJOR"));
pub const VERSION_MINER: u16 = utils::env_num(env!("CARGO_PKG_VERSION_MINOR"));
pub const SHAH_VERSION: (u16, u16) = (VERSION_MAJOR, VERSION_MINER);

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
    fn as_utf8_str_null_terminated(&self) -> &str;
}

impl AsUtf8Str for [u8] {
    fn as_utf8_str(&self) -> &str {
        match core::str::from_utf8(self) {
            Ok(v) => v,
            Err(e) => core::str::from_utf8(&self[..e.valid_up_to()])
                .unwrap_or_default(),
        }
    }
    fn as_utf8_str_null_terminated(&self) -> &str {
        let v = self.splitn(2, |x| *x == 0).next().unwrap_or_default();
        v.as_utf8_str()
    }
}

impl<const N: usize> AsUtf8Str for [u8; N] {
    fn as_utf8_str(&self) -> &str {
        self.as_slice().as_utf8_str()
    }
    fn as_utf8_str_null_terminated(&self) -> &str {
        self.as_slice().as_utf8_str_null_terminated()
    }
}
