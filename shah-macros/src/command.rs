use proc_macro::TokenStream;

pub(crate) fn command(args: TokenStream, code: TokenStream) -> TokenStream {
    let org = code.clone();
    let item = syn::parse_macro_input!(code as syn::ItemEnum);
    println!("item: {item:?}");

    org
}
