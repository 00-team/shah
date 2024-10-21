use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

pub(crate) fn command(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemEnum);
    println!("item: {item:#?}");

    let mut s = TokenStream2::new();

    s.into()
}
