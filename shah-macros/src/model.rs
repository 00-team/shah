use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use quote_into::quote_into;
use syn::parse::Parser;

// impl syn::parse::Parse for StrArgs {
//     fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
//         let result = StrArgs {get: false, set: false};
//         let key = input.parse().map(syn::Ident)?;
//         if key == "get" {}
//
//
//         Ok(Self {
//             set: false,
//             get: false
//         })
//     }
// }
//

type KeyVal = syn::punctuated::Punctuated<syn::ExprAssign, syn::Token![,]>;

pub(crate) fn model(_args: TokenStream, code: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(code as syn::ItemStruct);
    for attr in item.attrs.iter() {
        if let syn::Meta::List(meta) = &attr.meta {
            let ident = meta.path.segments[0].ident.to_string();
            if ident == "repr" {
                panic!("model must be repr(C) which is default")
            }
            if ident == "derive" {
                for token in meta.tokens.clone() {
                    if let proc_macro2::TokenTree::Ident(t) = token {
                        if t == "Default" {
                            panic!("remove the Default derive")
                        }
                    }
                }
            }
        }
    }
    item.attrs.push(syn::parse_quote! { #[repr(C)] });

    let ident = item.ident.clone();
    let mut asspad = TokenStream2::new();
    // let ci = crate::crate_ident();

    struct StrField {
        field: syn::Ident,
        get: bool,
        set: bool,
    }

    let mut str_fields = Vec::<StrField>::new();
    item.fields.iter_mut().for_each(|f| {
        for attr in f.attrs.iter_mut() {
            match &attr.meta {
                syn::Meta::Path(p) => {
                    if p.segments[0].ident == "str" {
                        str_fields.push(StrField {
                            field: f.ident.clone().unwrap(),
                            get: true,
                            set: true,
                        });
                    }
                }
                syn::Meta::List(l) => {
                    let parser = KeyVal::parse_terminated;
                    if l.path.segments[0].ident == "str" {
                        let mut sf = StrField {
                            field: f.ident.clone().unwrap(),
                            get: true,
                            set: true,
                        };
                        let args = match parser.parse(l.tokens.clone().into()) {
                            Ok(v) => v,
                            Err(e) => panic!("error parsing key value: {e}"),
                        };
                        let args = args.into_iter().filter_map(|a| {
                            if let syn::Expr::Path(p) = &(*a.left) {
                                if let syn::Expr::Lit(lit) = &(*a.right) {
                                    return Some((
                                        p.path.segments[0].ident.clone(),
                                        lit.lit.clone(),
                                    ));
                                }
                            }

                            panic!("invalid keyval args")
                        });
                        for (key, val) in args.into_iter() {
                            let val = if let syn::Lit::Bool(b) = val {
                                b.value
                            } else {
                                panic!("str args values must be bool")
                            };

                            if key == "set" {
                                sf.set = val;
                            }

                            if key == "get" {
                                sf.get = val;
                            }
                        }

                        str_fields.push(sf);
                    }
                }
                _ => {}
            }
        }
        f.attrs.clear();
    });

    let fields_len = item.fields.len();
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
                pub fn #set_ident(&mut self, value: &str) {
                    let vlen = value.len();
                    let flen = self.#field.len();
                    let len = if vlen > flen {
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
                    // let len = value.len().min(self.#field.len());
                    self.#field[..len].clone_from_slice(&value.as_bytes()[..len])
                }
            };
        }
    }

    quote! {
        #item

        const _: () = { #asspad };

        impl ::core::default::Default for #ident {
            #[inline]
            fn default() -> #ident {
                #default_impl
            }
        }

        impl #ident {
            #ssi
        }

        // impl #ci::FromBytes for #ident {
        //     fn from_bytes(data: &[u8]) -> Self {
        //         let data: [u8; <Self as #ci::Binary>::S] = data.try_into().unwrap();
        //         unsafe { core::mem::transmute(data) }
        //     }
        // }
    }
    .into()
}
