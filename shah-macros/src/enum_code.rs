use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote_into::quote_into;
use syn::{Fields, ItemEnum};

pub(crate) fn enum_code(_args: TokenStream, code: TokenStream) -> TokenStream {
    let output: TokenStream2 = code.clone().into();
    let item = syn::parse_macro_input!(code as ItemEnum);

    let mut s = TokenStream2::new();
    let ident = item.ident;
    let cb = quote! { { .. } };
    let pp = quote! { ( .. ) };
    let variants = item.variants.iter().enumerate().map(|(i, v)| {
        (
            i as u16,
            &v.ident,
            match v.fields {
                Fields::Unit => None,
                Fields::Named(_) => Some(&cb),
                Fields::Unnamed(_) => Some(&pp),
            },
        )
    });
    quote_into! {s +=
        #output

        impl From<#ident> for u16 {
            fn from(value: #ident) -> Self {
                match value {
                    #{variants.for_each(|(ix, vi, vf)|
                        quote_into!{s += #ident::#vi #vf => #ix,}
                    )}
                }
            }
        }
    };

    s.into()
}
