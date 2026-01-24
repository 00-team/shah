use crate::err;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use quote_into::quote_into;
use syn::spanned::Spanned;

pub(crate) fn model(mut item: syn::ItemStruct) -> syn::Result<TokenStream2> {
    let (impl_gnc, ty_gnc, where_gnc) = item.generics.split_for_impl();
    let is_generic = item.generics.lt_token.is_some();

    if !matches!(item.fields, syn::Fields::Named(_)) {
        return err!(item.span(), "invalid struct type must be named");
    }

    for attr in item.attrs.iter() {
        let syn::Meta::List(meta) = &attr.meta else {
            continue;
        };

        let ident = meta.path.segments[0].ident.to_string();
        if ident == "repr" {
            return err!(attr.span(), "model must be repr(C) which is default");
        }
        if ident == "derive" {
            for token in meta.tokens.clone() {
                let proc_macro2::TokenTree::Ident(t) = &token else {
                    continue;
                };

                match t.to_string().as_str() {
                    "Default" | "Copy" | "Clone" => {
                        return err!(
                            token.span(),
                            format!("remove the {t} derive")
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    item.attrs.push(syn::parse_quote! { #[derive(Copy)] });
    item.attrs.push(syn::parse_quote! { #[repr(C)] });

    let ident = item.ident.clone();
    let ci = crate::crate_ident();

    let mut assprv: Option<(&syn::Ident, &syn::Type)> = None;
    let mut asspad = TokenStream2::new();
    let assid = if is_generic { crate::ident!("Self") } else { ident.clone() };
    for f in item.fields.iter() {
        let Some(fid) = &f.ident else {
            return err!(f.span(), "field must have an ident");
        };
        let Some((pid, pty)) = assprv else {
            assprv = Some((fid, &f.ty));
            quote_into! { asspad +=
                assert!(::core::mem::offset_of!(#assid, #fid) == 0);
            }
            continue;
        };

        assprv = Some((fid, &f.ty));
        quote_into! { asspad +=
            assert!(::core::mem::offset_of!(#assid, #fid) ==
                ::core::mem::offset_of!(#assid, #pid) +
                ::core::mem::size_of::<#pty>()
            );
        }
    }

    if let Some((lid, lty)) = assprv {
        quote_into! { asspad +=
            assert!(
                ::core::mem::size_of::<#assid>() ==
                ::core::mem::offset_of!(#assid, #lid) +
                ::core::mem::size_of::<#lty>()
            );
        }
    }

    let mut default_impl = TokenStream2::new();
    for f in item.fields.iter() {
        let fi = &f.ident;
        match &f.ty {
            syn::Type::Path(_) => {
                quote_into!(default_impl += #fi: ::core::default::Default::default(),)
            }
            syn::Type::Array(a) => {
                let len = &a.len;
                let el = &a.elem;
                // let at = &path(&a.elem).path.segments[0].ident;
                quote_into!(default_impl += #fi: [<#el>::default(); #len],)
            }
            syn::Type::Tuple(t) => {
                quote_into! {default_impl += #fi: (#{
                    t.elems.iter().for_each(|e| quote_into!{default_impl += <#e>::default(),})
                }),}
            }
            t => {
                panic!("unknown type for default impl: {}", t.to_token_stream())
            }
        }
    }

    let mut s = quote! {
        #item

        #[automatically_derived]
        impl #impl_gnc ::core::default::Default for #ident #ty_gnc #where_gnc {
            #[inline]
            fn default() -> Self {
                #ident {#default_impl}
            }
        }

        #[automatically_derived]
        impl #impl_gnc Clone for #ident #ty_gnc #where_gnc {
            fn clone(&self) -> Self {
                *self
            }
        }

        #[automatically_derived]
        impl #impl_gnc #ci::models::Binary for #ident #ty_gnc #where_gnc {}

        #[automatically_derived]
        impl #impl_gnc #ci::ShahModel for #ident #ty_gnc #where_gnc {}
    };

    if is_generic {
        quote_into! {s +=
            impl #impl_gnc #ident #ty_gnc #where_gnc {
                pub const fn __assert_padding() {
                    #asspad
                }
            }
        };
    } else {
        quote_into! {s +=
            const _: () = { #asspad };
        };
    }

    Ok(s)
}
