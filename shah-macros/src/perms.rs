use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote_into::quote_into;

type IdentList = syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>;

pub fn perms(code: TokenStream) -> TokenStream {
    let ci = crate::crate_ident();
    let idents = syn::parse_macro_input!(code with IdentList::parse_terminated);

    let mut s = TokenStream2::new();

    for (idx, ident) in idents.iter().enumerate() {
        if ident.to_string().starts_with('_') {
            continue;
        }
        let (byte, bit) = (idx / 8, (idx % 8) as u8);
        quote_into! {s += pub const #ident: #ci::models::Perm = (#byte, #bit); };
    }

    s.into()
}
