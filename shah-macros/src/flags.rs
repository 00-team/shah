use crate::{err, utils::args::args_parse};
use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use quote_into::quote_into;
use syn::{parse::Parser, spanned::Spanned};

pub(crate) type Args =
    syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]>;

pub(crate) fn flags(
    args: Args, item: syn::ItemStruct,
) -> syn::Result<TokenStream2> {
    let mut s = TokenStream2::new();
    // let args = syn::parse::<syn::Meta>(args)?;
    // #[shah::flags(inner = [u8; 32], bits = 2, serde = "both")]

    let args = parse_args(args)?;
    let vis = &item.vis;
    let name = &item.ident;
    let inner = &args.inner;

    quote_into! {s +=
        #[repr(C)]
        #[derive(Debug, Default, Clone, Copy)]
        #vis struct #name {
            inner: #inner,
        }
    };

    Ok(s)
}

struct ParsedArgs {
    inner: syn::Type,
    bits: usize,
    serde_serialize: bool,
    serde_deserialize: bool,
}

fn parse_args(args: Args) -> syn::Result<ParsedArgs> {
    let mut pa = ParsedArgs {
        inner: syn::parse_quote!(u32),
        bits: 1,
        serde_serialize: false,
        serde_deserialize: false,
    };

    for a in args {
        const KEY_ERR: &str = "key must be one of: inner,bits,serde";
        let Some(id) = a.path.get_ident() else {
            return err!(a.path.span(), KEY_ERR);
        };
        match id.to_string().as_str() {
            "inner" => {
                match a.value {
                    syn::Expr::Repeat(v) => {}
                    syn::Expr::Path(v) => {
                        let Some(tp) = v.path.get_ident() else {
                            return err!(v.span(), format!("path: {v:#?}"));
                        };
                        // println!("path: {v:#?}");
                    }
                    v => {
                        return err!(
                            v.span(),
                            "only a type of [u8; N] or u8,u16,u32,u64 is allowed"
                        );
                    }
                };
                // let syn::Expr::Repeat(er) = a.value else
            }
            "bits" => {}
            "serde" => {}
            k => {
                return err!(
                    a.path.span(),
                    format!(
                        "unknown key of: {k}, must be one of: inner,bits,serde"
                    )
                );
            }
        }
    }

    Ok(pa)
}
