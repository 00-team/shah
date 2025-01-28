use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote_into::quote_into;
use syn::Fields;

pub(crate) fn enum_code(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let mut s = TokenStream2::new();

    let data = match item.data {
        syn::Data::Enum(de) => de,
        _ => panic!("EnumCode derive macro is only for enums"),
    };

    let cb = quote! { { .. } };
    let pp = quote! { ( .. ) };
    let variants = data.variants.iter().enumerate().map(|(i, v)| {
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
        impl From<#ident> for u16 {
            fn from(value: #ident) -> Self {
                Self::from(&value)
            }
        }
        impl From<&#ident> for u16 {
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
