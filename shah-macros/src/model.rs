use core::panic;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use quote_into::quote_into;
use syn::parse::Parser;

type KeyList = syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>;
type KeyVal = syn::punctuated::Punctuated<syn::ExprAssign, syn::Token![,]>;

pub(crate) fn model(_args: TokenStream, code: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(code as syn::ItemStruct);

    let generics = &item.generics;
    let is_generic = item.generics.lt_token.is_some();
    let mut gnb = item.generics.clone();
    for p in gnb.params.iter_mut() {
        match p {
            syn::GenericParam::Type(t) => t.bounds.clear(),
            _ => {
                panic!("invalid generic param")
            }
        }
    }

    match &item.fields {
        syn::Fields::Named(_) => {}
        _ => panic!("invalid struct type must be named"),
    }

    for attr in item.attrs.iter() {
        let syn::Meta::List(meta) = &attr.meta else {
            continue;
        };

        let ident = meta.path.segments[0].ident.to_string();
        if ident == "repr" {
            panic!("model must be repr(C) which is default")
        }
        if ident == "derive" {
            for token in meta.tokens.clone() {
                let proc_macro2::TokenTree::Ident(t) = token else {
                    continue;
                };

                if t == "Default" {
                    panic!("remove the Default derive")
                }
            }
        }
    }
    item.attrs.push(syn::parse_quote! { #[repr(C)] });

    let ident = item.ident.clone();
    let mut asspad = TokenStream2::new();
    let ci = crate::crate_ident();

    struct StrField {
        field: syn::Ident,
        get: bool,
        set: bool,
    }

    struct FlagField {
        field: syn::Ident,
        flags: Vec<syn::Ident>,
    }

    let mut flag_fields = Vec::<FlagField>::new();
    let mut str_fields = Vec::<StrField>::new();
    item.fields.iter_mut().for_each(|f| {
        f.attrs.retain(|attr| {
            match &attr.meta {
                syn::Meta::Path(p) => {
                    match p.segments[0].ident.to_string().as_str() {
                        "str" => {
                            str_fields.push(StrField {
                                field: f.ident.clone().unwrap(),
                                get: true,
                                set: true,
                            });
                            return false;
                        }
                        "flags" => {
                            panic!("flags attr must have at least one flag")
                        }
                        _ => {}
                    }
                }
                syn::Meta::List(l) => {
                    match l.path.segments[0].ident.to_string().as_str() {
                        "str" => {
                            let parser = KeyVal::parse_terminated;
                            let mut sf = StrField {
                                field: f.ident.clone().unwrap(),
                                get: true,
                                set: true,
                            };
                            let Ok(args) =
                                parser.parse(l.tokens.clone().into())
                            else {
                                panic!("error parsing key value")
                            };
                            let args = args.into_iter().map(|a| {
                                if let syn::Expr::Path(p) = &(*a.left) {
                                    if let syn::Expr::Lit(lit) = &(*a.right) {
                                        return (
                                            p.path.segments[0].ident.clone(),
                                            lit.lit.clone(),
                                        );
                                    }
                                }

                                panic!("invalid keyval args")
                            });
                            for (key, val) in args.into_iter() {
                                let syn::Lit::Bool(val) = val else {
                                    panic!("str args values must be bool")
                                };

                                match key.to_string().as_str() {
                                    "set" => sf.set = val.value,
                                    "get" => sf.get = val.value,
                                    _ => panic!("unknown key"),
                                }
                            }

                            str_fields.push(sf);
                            return false;
                        }
                        "flags" => {
                            let parser = KeyList::parse_terminated;
                            let keys = parser.parse(l.tokens.clone().into());
                            let Ok(keys) = keys else {
                                panic!("invalid #[flags] attr")
                            };

                            flag_fields.push(FlagField {
                                field: f.ident.clone().unwrap(),
                                flags: keys.iter().cloned().collect::<_>(),
                            });

                            return false;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            true
        });
    });

    let fields_len = item.fields.len();
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
    quote_into! {default_impl +=
        #ident {#{
            item.fields.iter().for_each(|f| {
                let fi = &f.ident;
                match &f.ty {
                    syn::Type::Path(_) => {
                        quote_into!(default_impl += #fi: ::core::default::Default::default(),)
                    },
                    syn::Type::Array(a) => {
                        let len = &a.len;
                        let el = &a.elem;
                        // let at = &path(&a.elem).path.segments[0].ident;
                        quote_into!(default_impl += #fi: [<#el>::default(); #len],)
                    }
                    t => {panic!("unknow type: {t:#?}")}
                }
            })
        }}
    }

    let mut ssi = TokenStream2::new();
    for StrField { field, get, set } in str_fields.iter() {
        if *get {
            quote_into! {ssi +=
                pub fn #field<'a>(&'a self) -> &'a str {
                    let value = self.#field.split(|c| *c == 0).next().unwrap();
                    match core::str::from_utf8(value) {
                        Err(e) => match core::str::from_utf8(&value[..e.valid_up_to()]) {
                            Ok(v) => v,
                            Err(_) => "",
                        },
                        Ok(v) => v,
                    }
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
    for FlagField { field, flags } in flag_fields.iter() {
        for (i, f) in flags.iter().enumerate() {
            let get = format_ident!("is_{f}");
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

    let s = quote! {
        #item

        const _: () = { #asspad };

        impl #generics ::core::default::Default for #ident #gnb {
            #[inline]
            fn default() -> #ident #gnb {
                #default_impl
            }
        }

        impl #generics #ident #gnb {
            #ssi

            #ffs
        }


        impl #generics #ci::Binary for #ident #gnb {}

        // impl #ci::FromBytes for #ident {
        //     fn from_bytes(data: &[u8]) -> Self {
        //         let data: [u8; <Self as #ci::Binary>::S] = data.try_into().unwrap();
        //         unsafe { core::mem::transmute(data) }
        //     }
        // }
    };

    // println!("{s}");

    s.into()
}
