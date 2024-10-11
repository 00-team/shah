use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use quote_into::quote_into;
use syn::{Field, ItemStruct, Meta, Type};

mod api;
mod enum_code;

#[proc_macro_attribute]
pub fn enum_code(args: TokenStream, code: TokenStream) -> TokenStream {
    enum_code::enum_code(args, code)
}

#[proc_macro_attribute]
pub fn api(args: TokenStream, code: TokenStream) -> TokenStream {
    api::api(args, code)
}

fn crate_ident() -> syn::Ident {
    // let found_crate = crate_name("shah").unwrap();
    // let name = match &found_crate {
    //     FoundCrate::Itself => "shah",
    //     FoundCrate::Name(name) => name,
    // };

    syn::Ident::new("shah", proc_macro2::Span::call_site())
}

// fn path(ty: &Type) -> &TypePath {
//     match ty {
//         Type::Path(p) => p,
//         _ => panic!("invalid type must be path"),
//     }
// }

#[proc_macro_attribute]
pub fn model(_args: TokenStream, code: TokenStream) -> TokenStream {
    // println!("code: {code}");
    let mut item = syn::parse_macro_input!(code as ItemStruct);
    for attr in item.attrs.iter() {
        if let Meta::List(meta) = &attr.meta {
            let ident = meta.path.segments[0].ident.to_string();
            if ident == "repr" {
                panic!("model must be repr(C) which is default")
            }
            if ident == "derive" {
                for token in meta.tokens.clone() {
                    if let TokenTree::Ident(t) = token {
                        if t == "Default" {
                            panic!("remove the Default derive")
                        }
                    }
                }
            }
        }
    }
    item.attrs.push(syn::parse_quote! { #[repr(C)] });
    // let crate_ident = crate_ident();

    let ident = item.ident.clone();
    let mut asspad = TokenStream2::new();

    let fields_len = item.fields.len();
    item.fields
        .iter()
        .enumerate()
        .scan(None as Option<&Field>, |state, (i, f)| {
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
                    Type::Path(_) => {
                        quote_into!(default_impl += #fi: ::core::default::Default::default(),)
                    },
                    Type::Array(a) => {
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

    // quote_into! {default_impl +=
    //     impl ::core::default::Default for #ident {
    //         #[inline]
    //         fn default() -> #ident {
    //             #ident {#{
    //                 item.fields.iter().for_each(|f| {
    //                     let fi = &f.ident;
    //                     match &f.ty {
    //                         Type::Path(_) => {
    //                             quote_into!(default_impl += #fi: ::core::default::Default::default(),)
    //                         },
    //                         Type::Array(a) => {
    //                             let len = &a.len;
    //                             let at = &path(&a.elem).path.segments[0].ident;
    //                             quote_into!(default_impl += #fi: [<#at>::default(), #len],)
    //                         }
    //                         t => {panic!("unknow type: {t:#?}")}
    //                     }
    //                 })
    //             }}
    //         }
    //     }
    // }

    quote! {
        #item

        const _: () = { #asspad };

        impl ::core::default::Default for #ident {
            #[inline]
            fn default() -> #ident {
                #default_impl
            }
        }

        // impl #ident {
        //     const SIZE: usize = ::core::mem::size_of::<#ident>();
        //
        //     fn from_bytes(value: [u8; <#ident>::SIZE]) -> Self {
        //         unsafe { ::core::mem::transmute(value) }
        //     }
        //
        //     fn into_bytes(&self) -> [u8; <#ident>::SIZE] {
        //         unsafe {
        //             ::core::slice::from_raw_parts(
        //                 self as *const #ident as *const u8,
        //                 <#ident>::SIZE,
        //             )
        //         }
        //         .try_into()
        //         .unwrap()
        //     }
        // }
    }
    .into()
}

#[proc_macro]
pub fn tuple_bytes_impl(_: TokenStream) -> TokenStream {
    // const ABC: &'static str = "ABCDEFGHIJKLMNOP";
    const ABC: &str = "ABC";
    let mut s = TokenStream2::new();

    for i in 0..ABC.len() {
        let iter = ABC.chars().map(|c| format_ident!("{c}")).take(i + 1);
        let mut out = TokenStream2::new();
        let mut total_size = TokenStream2::new();
        iter.clone().enumerate().for_each(|(j, c)| {
            quote_into!(out += #c,);
            quote_into!(total_size += <#c>::SIZE);
            if j < i {
                quote_into!(total_size += +);
            }
        });

        quote_into! {s +=
            // #[allow(unused_parens)]
            impl<#{iter.clone().for_each(|c|
                quote_into!(s += #c: crate::FromBytes,) )
            }> crate::FromBytes for (#out) {
                const SIZE: usize = #total_size;
                type Error = crate::error::Error;

                fn from_bytes(value: &[u8]) -> core::result::Result<Self, Self::Error> {
                    if value.len() != #total_size {
                        println!("invalid value size: {} must be {}", value.len(), #total_size);
                        return Err(Self::Error::shah(crate::error::PlutusError::Args));
                    }

                    Ok((#{
                        for (j, c) in iter.clone().enumerate() {
                            if j == 0 {
                                quote_into!(s += <#c>::from_bytes(&value[0..<#c>::SIZE]).map_err(Into::into)?,);
                            } else {
                                let mut prev = TokenStream2::new();
                                iter.clone().take(j).enumerate().for_each(|(k, c)| {
                                    quote_into!(prev += <#c>::SIZE);
                                    if k < j - 1 { quote_into!(prev += +); }
                                });
                                quote_into!(s += <#c>::from_bytes(&value[#prev..#prev + <#c>::SIZE]).map_err(Into::into)?,);
                            }
                        }
                    }))
                }
            }
        };
    }

    s.into()
}
