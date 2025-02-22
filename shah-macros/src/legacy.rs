use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote_into::quote_into;
use syn::{parse_quote, Fields, ItemEnum};



pub(crate) fn legacy(args: TokenStream, code: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(code as syn::ItemMod);

    println!("item: {item:#?}");

    let mut s = TokenStream2::new();

    s.into()
}



