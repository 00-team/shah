use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;
use quote_into::quote_into;
use syn::Fields;

pub(crate) fn enum_code(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let mut s = TokenStream2::new();

    let mut ecty = syn::Ident::new("u16", proc_macro2::Span::call_site());
    for attr in item.attrs.iter() {
        if let syn::Meta::List(ml) = &attr.meta {
            if ml.path.is_ident("enum_code") {
                ecty = syn::parse(ml.tokens.clone().into()).unwrap();
                break;
            }
        }
    }

    let data = match item.data {
        syn::Data::Enum(de) => de,
        _ => panic!("EnumCode derive macro is only for enums"),
    };

    let cb = quote! { { .. } };
    let pp = quote! { ( .. ) };
    let variants = data.variants.iter().enumerate().map(|(i, v)| {
        (
            Literal::usize_unsuffixed(i),
            &v.ident,
            match v.fields {
                Fields::Unit => None,
                Fields::Named(_) => Some(&cb),
                Fields::Unnamed(_) => Some(&pp),
            },
        )
    });
    quote_into! {s +=
        impl From<#ident> for #ecty {
            fn from(value: #ident) -> Self {
                Self::from(&value)
            }
        }
        impl From<&#ident> for #ecty {
            fn from(value: &#ident) -> Self {
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
