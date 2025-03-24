use crate::err;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote_into::quote_into;
use syn::spanned::Spanned;

// type Pargs = syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]>;

pub(crate) fn enum_int(
    mut item: syn::ItemEnum, args: TokenStream,
) -> syn::Result<TokenStream2> {
    let ty = syn::parse::<syn::Path>(args)?;

    for attr in item.attrs.iter() {
        if attr.path().is_ident("repr") {
            return err!(attr.span(), "remove the #[repr(...)] attr");
        }
    }

    item.attrs.push(syn::parse_quote! { #[repr(#ty)] });

    let mut brs = TokenStream2::new();
    let ident = &item.ident;
    // let mut br = TokenStream2::new();
    // let mut vars = Vec::<(usize, &syn::Ident)>::with_capacity(item.variants.len());
    let mut index = 0usize;
    let mut discr: Option<(&syn::Expr, Option<usize>)> = None;

    for v in item.variants.iter() {
        let syn::Fields::Unit = v.fields else {
            return err!(v.span(), "all variants must be a unit");
        };
        if let Some((_, exp)) = &v.discriminant {
            index = 0;
            if let syn::Expr::Lit(lit) = exp {
                let syn::Lit::Int(int) = &lit.lit else {
                    return err!(exp.span(), "only numbers are allowed");
                };
                let val = int.base10_parse::<usize>()?;
                discr = Some((exp, Some(val)));
            } else {
                discr = Some((exp, None));
            }
        }
        let vi = &v.ident;
        match discr {
            Some((e, None)) => {
                let idx = proc_macro2::Literal::usize_unsuffixed(index);
                quote_into!(brs += v if v == #e + #idx => Self::#vi,);
            }
            Some((_, Some(base))) => {
                let idx = proc_macro2::Literal::usize_unsuffixed(index + base);
                quote_into!(brs += #idx => Self::#vi,);
            }
            None => {
                let idx = proc_macro2::Literal::usize_unsuffixed(index);
                quote_into!(brs += #idx => Self::#vi,);
            }
        }
        index += 1;
    }

    // let variants = item.variants.iter().enumerate().map(|(i, v)| {
    //     v.discriminant
    //     if let syn::Fields::Unit = v.fields {
    //         return (Literal::isize_unsuffixed(start + i as isize), &v.ident);
    //     }
    //     panic!("all enum variants must be Unit")
    // });

    Ok(quote! {
        #item

        impl From<#ident> for #ty {
            fn from(value: #ident) -> Self {
                value as #ty
            }
        }

        impl From<#ty> for #ident {
            fn from(value: #ty) -> Self {
                match value {
                    #brs
                    // #{variants.for_each(|(x, v)| quote_into!(s += #x => Self::#v,))}
                    _ => Default::default(),
                }
            }
        }
    })
}
