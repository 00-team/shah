pub mod entity;
pub mod error;
pub(crate) mod utils;

mod binary;
pub mod server;
pub use crate::binary::Binary;
pub use crate::error::ErrorCode;
// use crate::error::PlutusError;

#[derive(Debug)]
pub struct Api<T> {
    pub name: &'static str,
    pub input_size: usize,
    pub output_size: usize,
    pub caller: fn(&mut T, &[u8], &mut [u8]) -> Result<(), ErrorCode>,
}

pub use shah_macros::{api, enum_code, model};

#[allow(unused_extern_crates)]
extern crate self as shah;

// shah_macros::tuple_bytes_impl!();

// [112, 108, 117, 116, 117, 115, 46]
// const SIGNATURE: [u8; 7] = *b"shah.";

// fn header(kind: u16, version: u16) -> [u8; 11] {
//     let mut header = [0; 11];
//     header[0..7].clone_from_slice(&SIGNATURE);
//     header[7..9].clone_from_slice(&kind.to_le_bytes());
//     header[9..11].clone_from_slice(&version.to_le_bytes());
//     header
// }

pub type GeneId = u64;

#[model]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Gene {
    pub id: GeneId,
    pub iter: u8,
    pub pepper: [u8; 3],
    pub server: u32,
}

pub trait Handler<Args>: Clone + 'static {
    type Output;

    fn call(&self, args: Args) -> Self::Output;
}

macro_rules! factory_tuple ({ $($param:ident)* } => {
    impl<Func, Out, $($param,)*> Handler<($($param,)*)> for Func
    where
        Func: Fn($($param),*) -> Out + Clone + 'static,
    {

        type Output = Out;

        #[inline]
        #[allow(non_snake_case)]
        fn call(&self, ($($param,)*): ($($param,)*)) -> Self::Output {
            (self)($($param,)*)
        }
    }
});

factory_tuple! {}
factory_tuple! { A }
factory_tuple! { A B }
// factory_tuple! { A B C }
// factory_tuple! { A B C D }
// factory_tuple! { A B C D E }
// factory_tuple! { A B C D E F }
// factory_tuple! { A B C D E F G }
// factory_tuple! { A B C D E F G H }
// factory_tuple! { A B C D E F G H I }
// factory_tuple! { A B C D E F G H I J }
// factory_tuple! { A B C D E F G H I J K }
// factory_tuple! { A B C D E F G H I J K L }
// factory_tuple! { A B C D E F G H I J K L M }
// factory_tuple! { A B C D E F G H I J K L M N }
// factory_tuple! { A B C D E F G H I J K L M N O }
// factory_tuple! { A B C D E F G H I J K L M N O P }

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn it_works() {
//     }
// }
