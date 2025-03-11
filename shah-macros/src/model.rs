use crate::{err, utils::args::args_parse};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use quote_into::quote_into;
use syn::{parse::Parser, spanned::Spanned};

type KeyList = syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>;
// type KeyVal = syn::punctuated::Punctuated<syn::ExprAssign, syn::Token![,]>;

pub(crate) fn model(mut item: syn::ItemStruct) -> syn::Result<TokenStream2> {
    let (impl_gnc, ty_gnc, where_gnc) = item.generics.split_for_impl();

    let is_generic = !item.generics.params.is_empty();

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

                if t == "Default" {
                    return err!(token.span(), "remove the Default derive");
                }
            }
        }
    }
    item.attrs.push(syn::parse_quote! { #[repr(C)] });

    let ident = item.ident.clone();
    let ci = crate::crate_ident();

    let mut flags_fields = Vec::<FlagsField>::with_capacity(5);
    let mut str_fields = Vec::<StrField>::with_capacity(5);

    for f in item.fields.iter_mut() {
        let mut retained_attrs = Vec::with_capacity(f.attrs.len());
        for attr in &f.attrs {
            let Some(ma) = parse_attrs(f, attr)? else {
                retained_attrs.push(attr.clone());
                continue;
            };
            match ma {
                ModelAttr::Str(v) => str_fields.push(v),
                ModelAttr::Flags(v) => flags_fields.push(v),
            }
        }

        f.attrs = retained_attrs;
    }

    let fields_len = item.fields.len();

    let mut asspad = TokenStream2::new();
    if !is_generic {
        item.fields
            .iter()
            .enumerate()
            .scan(None as Option<&syn::Field>, |state, (i, f)| {
                let field = f.ident.clone().unwrap();

                if let Some(prev) = state {
                    let pfi = prev.ident.clone().unwrap();
                    let pft = &prev.ty;
                    quote_into! { asspad +=
                        assert!(
                            ::core::mem::offset_of!(#ident, #field) ==
                            ::core::mem::offset_of!(#ident, #pfi) +
                            ::core::mem::size_of::<#pft>()
                        );
                    }
                } else {
                    quote_into! { asspad +=
                        assert!(::core::mem::offset_of!(#ident, #field) == 0);
                    }
                }

                if i == fields_len - 1 {
                    let ty = &f.ty;

                    quote_into! { asspad +=
                        assert!(
                            ::core::mem::size_of::<#ident>() ==
                            ::core::mem::offset_of!(#ident, #field) +
                            ::core::mem::size_of::<#ty>()
                        );
                    }
                }

                *state = Some(f);
                Some((i, f))
            })
            .for_each(|_| {});
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

    let mut ssi = TokenStream2::new();
    for StrField { field, get, set } in str_fields.iter() {
        if *get {
            quote_into! {ssi +=
                pub fn #field<'a>(&'a self) -> &'a str {
                    #ci::AsUtf8Str::as_utf8_str_null_terminated(&self.#field)
                }
            };
        }
        if *set {
            let set_ident = format_ident!("set_{field}");
            quote_into! {ssi +=
                pub fn #set_ident(&mut self, value: &str) -> bool {
                    let mut overflow = false;
                    let vlen = value.len();
                    let flen = self.#field.len();
                    let len = if vlen > flen {
                        overflow = true;
                        let mut idx = flen;
                        loop {
                            if value.is_char_boundary(idx) {
                                break idx;
                            }
                            idx -= 1;
                            continue;
                        }
                    } else {
                        vlen
                    };

                    self.#field[..len].clone_from_slice(&value.as_bytes()[..len]);
                    if len < flen {
                        self.#field[len] = 0;
                    }

                    overflow
                }
            };
        }
    }

    let mut ffs = TokenStream2::new();
    for FlagsField { field, flags, .. } in flags_fields.iter() {
        for (i, f) in flags.iter().enumerate() {
            let get = format_ident!("{f}");
            let set = format_ident!("set_{f}");
            quote_into! {ffs +=
                pub fn #get(&self) -> bool {
                    (self.#field & (1 << #i)) == (1 << #i)
                }

                pub fn #set(&mut self, #f: bool) -> &mut Self {
                    if #f {
                        self.#field |= (1 << #i);
                    } else {
                        self.#field &= !(1 << #i);
                    }
                    self
                }
            };
        }
    }

    Ok(quote! {
        #item

        const _: () = { #asspad };

        #[automatically_derived]
        impl #impl_gnc ::core::default::Default for #ident #ty_gnc #where_gnc {
            #[inline]
            fn default() -> Self {
                #ident {#default_impl}
            }
        }

        #[automatically_derived]
        impl #impl_gnc #ident #ty_gnc #where_gnc {
            #ssi

            #ffs
        }


        #[automatically_derived]
        impl #impl_gnc #ci::models::Binary for #ident #ty_gnc #where_gnc {}
    })
}

struct StrField {
    field: syn::Ident,
    get: bool,
    set: bool,
}

struct FlagsField {
    field: syn::Ident,
    flags: Vec<syn::Ident>,
    is_array: bool,
    bits: u8,
}

enum ModelAttr {
    Str(StrField),
    Flags(FlagsField),
}

args_parse! {
    #[derive(Debug)]
    struct StrArgs {
        get: Option<syn::LitBool>,
        set: Option<syn::LitBool>,
    }
}

fn parse_str_attr(
    f: &syn::Field, attr: &syn::Attribute,
) -> syn::Result<StrField> {
    const TYERR: &str = "#[str] field type must be an array of [u8; ..]";
    match &f.ty {
        syn::Type::Array(a) => match &(*a.elem) {
            syn::Type::Path(p) => {
                if !p.path.is_ident("u8") {
                    return err!(a.span(), TYERR);
                }
            }
            _ => return err!(a.span(), TYERR),
        },
        _ => return err!(f.ty.span(), TYERR),
    }

    match &attr.meta {
        syn::Meta::Path(_) => Ok(StrField {
            field: f.ident.clone().unwrap(),
            get: true,
            set: true,
        }),
        syn::Meta::List(ml) => {
            let args: StrArgs = syn::parse(ml.tokens.clone().into())?;
            Ok(StrField {
                field: f.ident.clone().unwrap(),
                get: args.get.map(|v| v.value).unwrap_or(true),
                set: args.set.map(|v| v.value).unwrap_or(true),
            })
        }
        _ => err!(attr.span(), "invalid attribute for #[str]"),
    }
}

fn parse_flags_attr(
    f: &syn::Field, attr: &syn::Attribute,
) -> syn::Result<FlagsField> {
    match &attr.meta {
        syn::Meta::List(ml) => {
            let parser = KeyList::parse_terminated;
            let keys = parser.parse(ml.tokens.clone().into());
            let Ok(keys) = keys else {
                return err!(attr.span(), "invalid #[flags] attr");
            };

            Ok(FlagsField {
                field: f.ident.clone().unwrap(),
                flags: keys.iter().cloned().collect::<_>(),
                is_array: false,
                bits: 1,
            })
        }
        _ => err!(attr.span(), "#[flags] attr must have at least one flag"),
    }
}

fn parse_attrs(
    f: &syn::Field, attr: &syn::Attribute,
) -> syn::Result<Option<ModelAttr>> {
    let attr_path = attr.path();
    if attr_path.is_ident("str") {
        return Ok(Some(ModelAttr::Str(parse_str_attr(f, attr)?)));
    }
    if attr_path.is_ident("flags") {
        return Ok(Some(ModelAttr::Flags(parse_flags_attr(f, attr)?)));
    }

    Ok(None)
}
