use core::panic;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, ToTokens};
use quote_into::quote_into;

use crate::crate_ident;

pub(crate) fn api(args: TokenStream, code: TokenStream) -> TokenStream {
    let original = code.clone();
    let mut s = TokenStream2::new();
    let api_mod = syn::parse_macro_input!(code as syn::ItemMod);
    if api_mod.content.is_none() {
        return original;
    }

    let attr = syn::parse_macro_input!(args as syn::Meta);
    let api_struct = match attr {
        syn::Meta::Path(p) => p.clone(),
        _ => panic!("invalid macro args. :/"),
    };

    let api_mod_idnet = &api_mod.ident;
    quote_into!(s+= mod #api_mod_idnet);

    let ci = crate_ident();
    let content = api_mod.content.unwrap().1;
    let api_funcs = content.iter().filter_map(|i| match i {
        syn::Item::Fn(f) => Some(&f.sig),
        _ => None,
    });

    #[derive(Debug)]
    struct Func {
        ident: syn::Ident,
        api_ident: syn::Ident,
        state: Option<syn::TypeReference>,
        inp: Vec<syn::Type>,
        out: Vec<syn::Type>,
    }
    let mut funcs = Vec::<Func>::with_capacity(api_funcs.clone().count());

    for sig in api_funcs {
        let mut func = Func {
            ident: sig.ident.clone(),
            api_ident: format_ident!("{}_api", sig.ident),
            state: None,
            inp: Default::default(),
            out: Default::default(),
        };
        let mut inp_done = false;

        for arg in sig.inputs.iter() {
            let arg = match arg {
                syn::FnArg::Typed(t) => t,
                _ => panic!("invalid function signature"),
            };

            match &(*arg.ty) {
                syn::Type::Reference(tr) => {
                    func.state = Some(tr.clone());
                }
                syn::Type::Tuple(tt) => {
                    tt.elems.iter().for_each(|t| {
                        if let syn::Type::Reference(ty) = t {
                            match &(*ty.elem) {
                                syn::Type::Path(_) => {}
                                syn::Type::Array(_) => {}
                                el => panic!(
                                    "invalid type: {}",
                                    el.to_token_stream()
                                ),
                            }
                            if !inp_done {
                                if ty.mutability.is_some() {
                                    panic!("input types must be immutable");
                                }

                                func.inp.push(*ty.elem.clone());
                            } else {
                                if ty.mutability.is_none() {
                                    panic!("output types must be mutable");
                                }

                                func.out.push(*ty.elem.clone());
                            }
                        } else {
                            panic!("invalid api")
                        }
                    });

                    inp_done = true;
                }
                ty => panic!("unknown api type: {}", ty.to_token_stream()),
            }
        }

        funcs.push(func);
    }

    quote_into! {s += {#{
        for item in content.iter() {
            quote_into! {s += #item};
        }

        for Func { api_ident, ident, state, inp, out } in funcs.iter() {
            let mut output_var = TokenStream2::new();
            for (i, t) in out.iter().enumerate() {
                let vid = format_ident!("ov{}", i);
                quote_into! {output_var +=
                    let (#vid, out) = out.split_at_mut(<#t as #ci::Binary>::S);
                    let #vid = <#t as #ci::Binary>::from_binary_mut(#vid);
                };
            }
            quote_into! {output_var += let output = (#{
                for (i, _) in out.iter().enumerate() {
                    let vid = format_ident!("ov{}", i);
                    quote_into!(output_var += #vid,)
                }
            });}

            let mut inp_before = TokenStream2::new();
            quote_into! {inp_before += 0};
            let mut input_var = TokenStream2::new();
            for t in inp.iter() {
                quote_into! {input_var += <#t as #ci::Binary>::from_binary(&inp[#inp_before..#inp_before + <#t as #ci::Binary>::S]),};
                quote_into! {inp_before += + <#t as #ci::Binary>::S};
            }

            quote_into! {s +=
                pub(crate) fn #api_ident(state: #state, inp: &[u8], out: &mut [u8]) -> Result<(), #ci::ErrorCode> {
                    let input = (#input_var);
                    #output_var
                    #ident(state, input, output)
                }
            };
        }

        let routes_len = funcs.len();
        quote_into! {s += pub(crate) const ROUTES: [#api_struct; #routes_len] = [#{
            for Func { api_ident, ident, inp, out, .. } in funcs.iter() {
                let mut inp_size = TokenStream2::new();
                quote_into!(inp_size += 0);
                inp.iter().for_each(|t| quote_into! {inp_size += + <#t as #ci::Binary>::S });

                let mut out_size = TokenStream2::new();
                quote_into!(out_size += 0);
                out.iter().for_each(|t| quote_into! {out_size += + <#t as #ci::Binary>::S });

                let name = ident.to_string();

                quote_into! {s += #api_struct {
                    name: #name,
                    caller: #api_ident,
                    input_size: #inp_size,
                    output_size: #out_size,
                },}
            }
        }];};

    }}};

    s.into()
}
