use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use quote_into::quote_into;
use syn::punctuated::Punctuated;

use crate::crate_ident;

type Args = Punctuated<syn::MetaNameValue, syn::Token![,]>;

pub(crate) fn api(args: TokenStream, code: TokenStream) -> TokenStream {
    let mut s = TokenStream2::new();
    let api_mod = syn::parse_macro_input!(code as syn::ItemMod);
    if api_mod.content.is_none() {
        return quote! {
            pub(crate) mod api {}
            pub mod client {}
        }
        .into();
    }
    let attrs = syn::parse_macro_input!(args with Args::parse_terminated);
    let ApiArgs { api_scope, api_struct, user_error } = parse_args(attrs);

    let ci = crate_ident();
    let content = api_mod.content.unwrap().1;
    let api_funcs = content.iter().filter_map(|i| match i {
        syn::Item::Fn(f) => Some(&f.sig),
        _ => None,
    });
    let api_uses = content.iter().filter(|i| matches!(i, syn::Item::Use(_)));

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

    quote_into! {s += pub(crate) mod api {#{
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

    quote_into! {s += pub mod client {#{
        for item in api_uses {
            quote_into! {s += #item};
        }

        for (route, Func { ident, inp, out, .. }) in funcs.iter().enumerate() {
            let inputs = inp.iter().enumerate().map(|(i, t)| (format_ident!("iv{i}"), t));
            // let outputs = out.iter().enumerate().map(|(i, t)| (format_ident!("ov{i}"), t));

            let mut input_size = TokenStream2::new();
            quote_into!(input_size += 0);
            inp.iter().for_each(|t| quote_into!(input_size += + <#t as #ci::Binary>::S));

            let mut bf = TokenStream2::new();
            quote_into! {bf += 0};
            let mut output_result = TokenStream2::new();
            for t in out.iter() {
                quote_into! {output_result += <#t as #ci::Binary>::from_binary(&reply_body[#bf..#bf + <#t as #ci::Binary>::S]),};
                quote_into! {bf += + <#t as #ci::Binary>::S};
            }

            let mut bf = TokenStream2::new();
            quote_into! {bf += 0};
            let mut input_result = TokenStream2::new();
            for (i, t) in inputs.clone() {
                quote_into! {input_result += order_body[#bf..#bf + <#t as #ci::Binary>::S].clone_from_slice(<#t as #ci::Binary>::as_binary(#i));};
                quote_into! {bf += + <#t as #ci::Binary>::S};
            }


            quote_into! {s +=
                pub fn #ident<'a>(
                    taker: &'a mut #ci::Taker,
                    #{inputs.clone().for_each(|(i, t)| quote_into!(s += #i: &#t, ))}
                ) -> Result<(#{out.iter().for_each(|t| quote_into!(s += &'a #t,))}), #ci::ClientError<#user_error>> {
                // ) -> Result<(), #ci::ClientError<#user_error>> {
                    let mut order = [0u8; #input_size + <#ci::OrderHead as #ci::Binary>::S];
                    let (order_head, order_body) = order.split_at_mut(<#ci::OrderHead as #ci::Binary>::S);
                    let order_head = <#ci::OrderHead as #ci::Binary>::from_binary_mut(order_head);
                    order_head.scope = #api_scope as u8;
                    order_head.route = #route as u8;
                    order_head.size = #input_size as u32;

                    #input_result

                    taker.take(&order)?;
                    let reply_head = taker.reply_head();
                    let reply_body = taker.reply_body(reply_head.size as usize);
                    Ok((#output_result))
                }
            }
        }
    }}};

    s.into()
}

struct ApiArgs {
    api_struct: syn::Path,
    user_error: syn::Path,
    api_scope: syn::LitInt,
}

fn parse_args(args: Args) -> ApiArgs {
    let mut api_struct: Option<syn::Path> = None;
    let mut user_error: Option<syn::Path> = None;
    let mut api_scope: Option<syn::LitInt> = None;

    for meta in args.iter() {
        let key = meta.path.segments[0].ident.to_string();
        match key.as_str() {
            "scope" => {
                if let syn::Expr::Lit(lit) = &meta.value {
                    if let syn::Lit::Int(int) = &lit.lit {
                        api_scope = Some(int.clone());
                    }
                }
            }
            "api" => {
                if let syn::Expr::Path(path) = &meta.value {
                    api_struct = Some(path.path.clone());
                }
            }
            "error" => {
                if let syn::Expr::Path(path) = &meta.value {
                    user_error = Some(path.path.clone());
                }
            }
            _ => {}
        }
    }

    if api_struct.is_none() || api_scope.is_none() || user_error.is_none() {
        panic!("invalid attrs. api = <Path>, scope = usize, error = UserError")
    }

    ApiArgs {
        api_struct: api_struct.unwrap(),
        user_error: user_error.unwrap(),
        api_scope: api_scope.unwrap(),
    }
}
