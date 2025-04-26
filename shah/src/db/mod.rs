pub mod apex;
pub mod belt;
pub mod entity;
pub mod pond;
pub mod snake;
pub mod trie;
pub mod trie_const;

macro_rules! derr {
    ($ls:expr, $err:expr) => {{
        log::error!("{} derr: {:?}", $ls, $err);
        Err($err)?
    }};
}
pub(crate) use derr;
