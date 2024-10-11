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

    // println!("s: {}, funcs: {funcs:#?}", s);

    // original
    // s.into()
    // let mut s = TokenStream2::new();
    //
    // const N: usize = 3;
    // let abc = [
    //     'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N',
    //     'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    // ]
    // .iter()
    // .map(|u| {
    //     (
    //         format_ident!("{}", u),
    //         format_ident!("{}", u.to_lowercase().to_string()),
    //     )
    // })
    // .collect::<Vec<_>>();
    //
    // for i in 0..N {
    //     for j in 0..N {
    //         let inp = (0..i).map(|x| &abc[x]);
    //         let out = (0..j).map(|x| &abc[i + x]);
    //         let all = (0..i + j).map(|x| &abc[x]);
    //
    //         let mut input_size = TokenStream2::new();
    //         quote_into! {input_size += 0 #{inp.clone().for_each(|(u, _)| quote_into!(input_size += + <#u as #ci::Binary>::S))}};
    //
    //         let mut output_size = TokenStream2::new();
    //         quote_into! {output_size += 0 #{out.clone().for_each(|(u, _)| quote_into!(output_size += + <#u as #ci::Binary>::S))}};
    //
    //         let mut itu = TokenStream2::new();
    //         let mut itl = TokenStream2::new();
    //         inp.clone().for_each(|(u, l)| {
    //             quote_into! {itu += &#u,};
    //             quote_into! {itl += #l,};
    //         });
    //
    //         let mut otu = TokenStream2::new();
    //         let mut otl = TokenStream2::new();
    //         out.clone().for_each(|(u, l)| {
    //             quote_into! {otu += &mut #u,}
    //             quote_into! {otl += #l,}
    //         });
    //
    //         let mut at = TokenStream2::new();
    //         all.clone().for_each(|(u, _)| quote_into! {at += #u,});
    //
    //         // let mut out_before = TokenStream2::new();
    //         // quote_into! {out_before += 0};
    //         let mut output_var = TokenStream2::new();
    //         for (u, l) in out {
    //             quote_into! {output_var +=
    //                 let (#l, out) = out.split_at_mut(<#u as #ci::Binary>::S);
    //                 let #l = <#u as #ci::Binary>::from_binary_mut(#l);
    //             };
    //             // quote_into! {output_var += <#x as #ci::Binary>::from_binary_mut(&mut out[#out_before..#out_before + <#x as #ci::Binary>::S]),};
    //             // quote_into! {out_before += + <#x as #ci::Binary>::S};
    //         }
    //
    //         let mut inp_before = TokenStream2::new();
    //         quote_into! {inp_before += 0};
    //         let mut input_var = TokenStream2::new();
    //         for (x, _) in inp {
    //             // quote_into! {input_var +=
    //             //     let (#l, in) = out.split_at_mut(<#u as #ci::Binary>::S);
    //             //     let #l = <#u as #ci::Binary>::from_binary_mut(#l);
    //             // };
    //
    //             quote_into! {input_var += <#x as #ci::Binary>::from_binary(&inp[#inp_before..#inp_before + <#x as #ci::Binary>::S]),};
    //             quote_into! {inp_before += + <#x as #ci::Binary>::S};
    //         }
    //
    //         quote_into! {s +=
    //             impl<Func, Db, #at> Api<Db, (#itu), (#otu)> for Func
    //             where
    //                 Func: Fn(Db, (#itu), (#otu)) -> Result<(), #ci::ErrorCode>,
    //                 #{all.for_each(|(x, _)| quote_into!(s+= #x: #ci::Binary,))}
    //             {
    //                 const INPUT_SIZE: usize = #input_size;
    //                 const OUTPUT_SIZE: usize = #output_size;
    //
    //                 #[inline]
    //                 fn api(&self, db: Db, inp: &[u8], out: &mut [u8]) -> Result<(), #ci::ErrorCode> {
    //                     if inp.len() != Self::INPUT_SIZE {
    //                         Err(#ci::PlutusError::BadInputLength)?;
    //                     }
    //
    //                     #output_var
    //
    //                     let input = (#input_var);
    //                     self(db, input, (#otl))
    //                 }
    //             }
    //         };
    //     }
    // }
    // s.into()
}
