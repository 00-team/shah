use proc_macro::TokenStream;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote_into::quote_into;
use syn::{parse_quote, Fields, ItemEnum};

type Pargs = syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]>;

pub(crate) fn enum_int(args: TokenStream, code: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(code as ItemEnum);
    let pargs = syn::parse_macro_input!(args with Pargs::parse_terminated);
    let Args { ty, start } = parse_pargs(pargs);

    for attr in item.attrs.iter() {
        if let syn::Meta::List(ml) = &attr.meta {
            if ml.path.segments[0].ident == "repr" {
                panic!("remove the #[repr(...)] attribute");
            }
        }
    }

    item.attrs.push(syn::parse_quote! { #[repr(#ty)] });

    let mut s = TokenStream2::new();
    let ident = &item.ident;
    let variants = item.variants.iter().enumerate().map(|(i, v)| {
        if let Fields::Unit = v.fields {
            return (Literal::isize_unsuffixed(start + i as isize), &v.ident);
        }
        panic!("all enum variants must be Unit")
    });
    quote_into! {s +=
        #item

        impl From<#ident> for #ty {
            fn from(value: #ident) -> Self {
                value as #ty
                // match value {
                //     #{variants.for_each(|(ix, vi, vf)|
                //         quote_into!{s += #ident::#vi #vf => #ix,}
                //     )}
                // }
            }
        }

        impl From<#ty> for #ident {
            fn from(value: #ty) -> Self {
                match value {
                    #{variants.for_each(|(x, v)| quote_into!(s += #x => Self::#v,))}
                    _ => Default::default(),
                }
            }
        }
    };

    s.into()
}

struct Args {
    ty: syn::Path,
    start: isize,
}

fn parse_pargs(pargs: Pargs) -> Args {
    let mut args = Args {
        // ty: syn::Path { leading_colon: None, segments: Default::default() },
        ty: parse_quote!(u8),
        start: 0,
    };
    // args.ty.segments.push(syn::PathSegment {
    //     ident: format_ident!("u8"),
    //     arguments: Default::default(),
    // });

    for meta in pargs.iter() {
        let key = meta.path.segments[0].ident.to_string();
        match key.as_str() {
            "start" => {
                if let syn::Expr::Lit(lit) = &meta.value {
                    if let syn::Lit::Int(int) = &lit.lit {
                        args.start =
                            int.base10_parse().expect("invalid start value");
                    }
                }
            }
            "ty" => {
                if let syn::Expr::Path(path) = &meta.value {
                    args.ty = path.path.clone();
                }
            }
            _ => {}
        }
    }

    args
}
