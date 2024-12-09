use crate::crate_ident;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, ToTokens};
use quote_into::quote_into;
use syn::punctuated::Punctuated;

type Args = Punctuated<syn::MetaNameValue, syn::Token![,]>;

#[derive(Debug)]
struct Route {
    ident: syn::Ident,
    api_ident: syn::Ident,
    inp: Vec<syn::Type>,
    out: Vec<syn::Type>,
    ret: bool,
    doc: String,
}

pub(crate) fn api(args: TokenStream, code: TokenStream) -> TokenStream {
    let mut s = TokenStream2::new();
    let item = syn::parse_macro_input!(code as syn::ItemMod);
    let Some((_, content)) = item.content else {
        panic!("invalid api mod");
    };
    let attrs = syn::parse_macro_input!(args with Args::parse_terminated);
    let ApiArgs { api_scope, user_error } = parse_args(attrs);
    let ci = crate_ident();
    let mut uses = TokenStream2::new();
    let mut user_funcs = Vec::<syn::ItemFn>::new();
    let mut user_client = TokenStream2::new();

    for item in content.iter() {
        match &item {
            syn::Item::Fn(f) => {
                let mut f = f.clone();
                let is_client = f
                    .attrs
                    .iter_mut()
                    .any(|a| a.meta.to_token_stream().to_string() == "client");
                if is_client {
                    f.attrs.clear();
                    quote_into!(user_client += #f);
                } else {
                    user_funcs.push(f);
                }
            }
            syn::Item::Use(u) => quote_into!(uses += #u),
            _ => panic!(
                "unknown item: {} was found in api mod",
                item.to_token_stream()
            ),
        }
    }

    let mut routes = Vec::<Route>::with_capacity(user_funcs.len());
    let mut state: Option<syn::Type> = None;

    for syn::ItemFn { sig, .. } in user_funcs.iter() {
        let mut route = Route {
            ident: sig.ident.clone(),
            api_ident: format_ident!("{}_api", sig.ident),
            inp: Default::default(),
            out: Default::default(),
            ret: returns_output_size(&sig.output),
            doc: "input: ".to_string(),
        };
        let mut inp_done = false;

        for arg in sig.inputs.iter() {
            let arg = match arg {
                syn::FnArg::Typed(t) => t,
                _ => panic!("invalid function signature"),
            };

            match &(*arg.ty) {
                syn::Type::Reference(tr) => {
                    if state.is_none() {
                        state = Some(*tr.elem.clone());
                    }
                }
                syn::Type::Tuple(tt) => {
                    route.doc += &arg.pat.to_token_stream().to_string();
                    tt.elems.iter().for_each(|t| {
                        let syn::Type::Reference(ty) = t else {
                            panic!("invalid api")
                        };

                        match &(*ty.elem) {
                            syn::Type::Path(_) => {}
                            syn::Type::Array(_) => {}
                            syn::Type::Slice(_) => {
                                panic!(
                                    "dynamic data (aka slice) is not supported"
                                )
                            }
                            el => {
                                panic!("invalid type: {}", el.to_token_stream())
                            }
                        }
                        if !inp_done {
                            if ty.mutability.is_some() {
                                panic!("input types must be immutable");
                            }

                            route.inp.push(*ty.elem.clone());
                        } else {
                            if ty.mutability.is_none() {
                                panic!("output types must be mutable");
                            }

                            route.out.push(*ty.elem.clone());
                        }
                    });

                    if !inp_done {
                        route.doc += "\noutput: ";
                    }
                    inp_done = true;
                }
                ty => panic!("unknown api type: {}", ty.to_token_stream()),
            }
        }

        routes.push(route);
    }

    quote_into! {s += pub(crate) mod api {
        #![allow(unused_imports)]

        #uses

    #{
        for f in user_funcs.iter() {
            quote_into!(s += #f);
        }

        for Route { api_ident, ident, inp, out, ret, .. } in routes.iter() {
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

            let mut bf = TokenStream2::new();
            quote_into! {bf += 0};
            let mut input_var = TokenStream2::new();
            for t in inp.iter() {
                quote_into! {input_var += <#t as #ci::Binary>::from_binary(&inp[#bf..#bf + <#t as #ci::Binary>::S]),};
                quote_into! {bf += + <#t as #ci::Binary>::S};
            }

            let mut out_size = TokenStream2::new();
            quote_into!(out_size += 0);
            out.iter().for_each(|t| quote_into! {out_size += + <#t as #ci::Binary>::S });

            quote_into! {s +=
                #[allow(dead_code)]
                pub(crate) fn #api_ident(state: &mut #state, inp: &[u8], out: &mut [u8]) -> Result<usize, #ci::ErrorCode> {
                    let input = (#input_var);
                    #output_var
                    let res = #ident(state, input, output)?;
                    #{if *ret {
                        quote_into!(s += Ok(res))
                    } else {
                        quote_into!(s += Ok(#out_size))
                    }}
                }
            };
        }

        let routes_len = routes.len();
        quote_into! {s += pub(crate) const ROUTES: [#ci::Api<#state>; #routes_len] = [#{
            for Route { api_ident, ident, inp, .. } in routes.iter() {
                let mut input_size = TokenStream2::new();
                quote_into!(input_size += 0);

                for t in inp.iter() {
                    quote_into! {input_size += + <#t as #ci::Binary>::S }
                }

                let name = ident.to_string();
                quote_into! {s += #ci::Api::<#state> {
                    name: #name,
                    caller: #api_ident,
                    input_size: #input_size,
                },}
            }
        }];};
    }

        pub(crate) const FILE: &str = file!();
        pub(crate) const SCOPE: usize = #api_scope;
    }};

    let mut c = TokenStream2::new();
    for (rdx, Route { ident, inp, out, doc, .. }) in routes.iter().enumerate() {
        let inputs = inp
            .iter()
            .enumerate()
            .map(|(idx, ty)| (format_ident!("iv{idx}"), ty));
        // let outputs = out.iter().enumerate().map(|(i, t)| (format_ident!("ov{i}"), t));

        let mut input_size = TokenStream2::new();
        quote_into!(input_size += 0);
        for t in inp.iter() {
            quote_into!(input_size += + <#t as #ci::Binary>::S);
        }

        let mut bf = TokenStream2::new();
        quote_into! {bf += 0};
        let mut output_result = TokenStream2::new();
        for t in out.iter() {
            quote_into! {output_result += <#t as #ci::Binary>::from_binary(&reply.body[#bf..#bf + <#t as #ci::Binary>::S]).clone(),};
            quote_into! {bf += + <#t as #ci::Binary>::S};
        }

        let mut bf = TokenStream2::new();
        quote_into! {bf += 0};
        let mut input_result = TokenStream2::new();
        for (i, t) in inputs.clone() {
            quote_into! {input_result += order_body[#bf..#bf + <#t as #ci::Binary>::S].clone_from_slice(<#t as #ci::Binary>::as_binary(#i));};
            quote_into! {bf += + <#t as #ci::Binary>::S};
        }

        quote_into! {c +=
            #[doc = #doc]
            pub fn #ident(
                taker: &#ci::Taker,
                #{inputs.clone().for_each(|(i, t)| quote_into!(c += #i: &#t, ))}
            ) -> Result<(#{out.iter().for_each(|t| quote_into!(c += #t,))}), #ci::ClientError<#user_error>> {
            // ) -> Result<(), #ci::ClientError<#user_error>> {
                let mut order = [0u8; #input_size + <#ci::OrderHead as #ci::Binary>::S];
                let (order_head, order_body) = order.split_at_mut(<#ci::OrderHead as #ci::Binary>::S);
                let order_head = <#ci::OrderHead as #ci::Binary>::from_binary_mut(order_head);
                order_head.scope = #api_scope as u8;
                order_head.route = #rdx as u8;
                order_head.size = (#input_size) as u32;

                #input_result

                let reply = taker.take(&mut order)?;
                // let reply_head = taker.reply_head();
                // let reply_body = taker.reply_body(reply_head.size as usize);
                Ok((#output_result))
            }
        }
    }

    quote_into! {s += pub mod client {
        #![allow(dead_code, unused_imports)]

        #uses
        #c
        #user_client
    }};

    s.into()
}

struct ApiArgs {
    user_error: syn::Path,
    api_scope: syn::LitInt,
}

fn parse_args(args: Args) -> ApiArgs {
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
            "error" => {
                if let syn::Expr::Path(path) = &meta.value {
                    user_error = Some(path.path.clone());
                }
            }
            _ => {}
        }
    }

    if api_scope.is_none() || user_error.is_none() {
        panic!("invalid attrs. api = <Path>, scope = usize, error = UserError")
    }

    ApiArgs { user_error: user_error.unwrap(), api_scope: api_scope.unwrap() }
}

fn returns_output_size(rt: &syn::ReturnType) -> bool {
    'b: {
        let syn::ReturnType::Type(_, t) = rt else { break 'b };

        let syn::Type::Path(p) = &(**t) else { break 'b };
        let args = &p.path.segments[0].arguments;

        let syn::PathArguments::AngleBracketed(a) = args else { break 'b };
        let syn::GenericArgument::Type(t) = &a.args[0] else { break 'b };

        if let syn::Type::Tuple(tp) = t {
            if tp.elems.is_empty() {
                return false;
            }
        }

        if let syn::Type::Path(p) = t {
            if p.to_token_stream().to_string() == "usize" {
                return true;
            }
        }

        break 'b;
    }

    panic!("return type of an api must be Result<(), ErrorCode> or Result<usize, ErrorCode>")
}
