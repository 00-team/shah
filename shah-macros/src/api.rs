use crate::err;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use quote_into::quote_into;
use syn::{punctuated::Punctuated, spanned::Spanned};

type Args = Punctuated<syn::MetaNameValue, syn::token::Comma>;

pub(crate) fn api(args: Args, item: syn::ItemMod) -> syn::Result<TokenStream2> {
    let item_span = item.span();
    let Some((_, content)) = item.content else {
        return err!(item_span, "mod is empty");
    };

    if content.is_empty() {
        return err!(item_span, "mod is empty");
    }

    let mut s = TokenStream2::new();

    let ApiArgs { api_scope, user_error } = parse_args(args)?;
    let ci = crate::crate_ident();
    let mut uses = TokenStream2::new();
    let mut user_funcs = Vec::<syn::ItemFn>::new();
    let mut user_client = TokenStream2::new();
    let bin = quote! { #ci::models::Binary };

    for item in content.iter() {
        match &item {
            syn::Item::Fn(f) => {
                let mut f = f.clone();
                if f.attrs.iter().any(|a| a.path().is_ident("client")) {
                    f.attrs.clear();
                    quote_into!(user_client += #f);
                } else {
                    user_funcs.push(f);
                }
            }
            syn::Item::Use(u) => quote_into!(uses += #u),
            _ => return err!(item.span(), "only fn's and use's are valid"),
        }
    }

    let mut routes = Vec::<Route>::with_capacity(user_funcs.len());
    let mut state: Option<syn::Type> = None;

    for syn::ItemFn { sig, .. } in user_funcs.iter() {
        let r = Route::from_signature(sig)?;
        if let Some(ref s) = state {
            if s != &r.state {
                return err!(
                    sig.span(),
                    "state type does not match previous instances of this type"
                );
            }
        } else {
            state = Some(r.state.clone());
        }
        routes.push(r);
    }

    // TODO: if output is only one item which very common. turn the
    // return (A, ) into just return A; which way nicer to work with

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
                    let (#vid, out) = out.split_at_mut(<#t as #bin>::S);
                    let #vid = <#t as #bin>::from_binary_mut(#vid);
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
                quote_into! {input_var += <#t as #bin>::from_binary(&inp[#bf..#bf + <#t as #bin>::S]),};
                quote_into! {bf += + <#t as #bin>::S};
            }

            let mut out_size = TokenStream2::new();
            quote_into!(out_size += 0);
            out.iter().for_each(|t| quote_into! {out_size += + <#t as #bin>::S });

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
        quote_into! {s += pub(crate) const ROUTES: [#ci::models::Api<#state>; #routes_len] = [#{
            for Route { api_ident, ident, inp, .. } in routes.iter() {
                let mut input_size = TokenStream2::new();
                quote_into!(input_size += 0);

                for t in inp.iter() {
                    quote_into! {input_size += + <#t as #bin>::S }
                }

                let name = ident.to_string();
                quote_into! {s += #ci::models::Api::<#state> {
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
            quote_into!(input_size += + <#t as #bin>::S);
        }

        let mut bf = TokenStream2::new();
        quote_into! {bf += 0};
        let mut output_result = TokenStream2::new();
        for t in out.iter() {
            quote_into! {output_result += <#t as #bin>::from_binary(&reply.body[#bf..#bf + <#t as #bin>::S]).clone(),};
            quote_into! {bf += + <#t as #bin>::S};
        }

        let mut bf = TokenStream2::new();
        quote_into! {bf += 0};
        let mut input_result = TokenStream2::new();
        for (i, t) in inputs.clone() {
            quote_into! {input_result += order_body[#bf..#bf + <#t as #bin>::S].clone_from_slice(<#t as #bin>::as_binary(#i));};
            quote_into! {bf += + <#t as #bin>::S};
        }

        quote_into! {c +=
            #[doc = #doc]
            pub fn #ident(
                taker: &#ci::Taker,
                #{inputs.clone().for_each(|(i, t)| quote_into!(c += #i: &#t, ))}
            ) -> Result<(#{out.iter().for_each(|t| quote_into!(c += #t,))}), #ci::ClientError<#user_error>> {
            // ) -> Result<(), #ci::ClientError<#user_error>> {
                let mut order = [0u8; #input_size + <#ci::models::OrderHead as #bin>::S];
                let (order_head, order_body) = order.split_at_mut(<#ci::models::OrderHead as #bin>::S);
                let order_head = <#ci::models::OrderHead as #bin>::from_binary_mut(order_head);
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

    Ok(s)
}

struct ApiArgs {
    user_error: syn::Path,
    api_scope: syn::LitInt,
}

fn parse_args(args: Args) -> syn::Result<ApiArgs> {
    let mut user_error: Option<syn::Path> = None;
    let mut api_scope: Option<syn::LitInt> = None;

    for meta in args.iter() {
        let key = meta.path.segments[0].ident.to_string();
        match key.as_str() {
            "scope" => {
                if api_scope.is_some() {
                    return err!(
                        meta.span(),
                        "you cannot set `scope` multiple times"
                    );
                }
                api_scope =
                    Some(syn::parse(meta.value.to_token_stream().into())?);
            }
            "error" => {
                if user_error.is_some() {
                    return err!(
                        meta.span(),
                        "you cannot set `error` multiple times"
                    );
                }
                if let syn::Expr::Path(path) = &meta.value {
                    user_error = Some(path.path.clone());
                }
            }
            _ => {}
        }
    }

    let Some(api_scope) = api_scope else {
        return err!(
            args.span(),
            "no scope = <num> was found in macro attributes"
        );
    };

    let Some(user_error) = user_error else {
        return err!(
            args.span(),
            "no error = YourError was found in macro attributes"
        );
    };

    Ok(ApiArgs { user_error, api_scope })
}

fn returns_output_size(rt: &syn::ReturnType) -> syn::Result<bool> {
    macro_rules! e {
        () => {
            err!(
                rt.span(),
                "return type of an api must be Result<(), ErrorCode> ",
                "or Result<usize, ErrorCode>"
            )
        };
    }

    let syn::ReturnType::Type(_, t) = rt else { return e!() };

    let syn::Type::Path(p) = &(**t) else { return e!() };
    let args = &p.path.segments[0].arguments;

    let syn::PathArguments::AngleBracketed(a) = args else { return e!() };
    let syn::GenericArgument::Type(t) = &a.args[0] else { return e!() };

    if let syn::Type::Tuple(tp) = t {
        if tp.elems.is_empty() {
            return Ok(false);
        }
    }

    if let syn::Type::Path(p) = t {
        if p.to_token_stream().to_string() == "usize" {
            return Ok(true);
        }
    }

    e!()
}

#[derive(Debug)]
struct Route {
    state: syn::Type,
    ident: syn::Ident,
    api_ident: syn::Ident,
    inp: Vec<syn::Type>,
    out: Vec<syn::Type>,
    ret: bool,
    doc: String,
}

impl Route {
    fn from_signature(sig: &syn::Signature) -> syn::Result<Self> {
        let mut route = Route {
            state: syn::Type::Never(syn::TypeNever {
                bang_token: Default::default(),
            }),
            ident: sig.ident.clone(),
            api_ident: format_ident!("{}_api", sig.ident),
            inp: Default::default(),
            out: Default::default(),
            ret: returns_output_size(&sig.output)?,
            doc: "input: ".to_string(),
        };

        if sig.inputs.len() != 3 {
            return err!(
                sig.inputs.span(),
                "api functions requires 3 arguments ",
                "fn my_api(state: &mut State, inputs: (&A, &B), ",
                "outputs: (&mut C, &mut D))"
            );
        }

        fn typed(a: &syn::FnArg) -> syn::Result<&syn::PatType> {
            let syn::FnArg::Typed(pt) = a else {
                return err!(a.span(), "invalid fn arg :/");
            };
            Ok(pt)
        }

        let state = typed(&sig.inputs[0])?;
        let syn::Type::Reference(s) = &(*state.ty) else {
            return err!(state.span(), "state type must be a &mut MyState");
        };
        route.state = *s.elem.clone();

        fn tup(a: &syn::PatType, mm: bool) -> syn::Result<Vec<syn::Type>> {
            let syn::Type::Tuple(tt) = &(*a.ty) else {
                return err!(a.span(), "input and output types must be tuple");
            };
            let mut tarr = Vec::<syn::Type>::with_capacity(tt.elems.len());
            for t in tt.elems.iter() {
                let syn::Type::Reference(tr) = t else {
                    return err!(
                        t.span(),
                        "input/output tuple elements must be a reference"
                    );
                };

                if mm && tr.mutability.is_none() {
                    return err!(
                        t.span(),
                        "output elements must mutable references"
                    );
                }

                if !mm && tr.mutability.is_some() {
                    return err!(
                        t.span(),
                        "input elements must immutable references"
                    );
                }

                match &(*tr.elem) {
                    syn::Type::Path(_) | syn::Type::Array(_) => {}
                    syn::Type::Slice(_) => {
                        return err!(
                            tr.elem.span(),
                            "slices are not supported yet!"
                        );
                    }
                    _ => {
                        return err!(tr.elem.span(), "unknown type was found.");
                    }
                }

                tarr.push(*tr.elem.clone());
            }
            Ok(tarr)
        }

        let inp = typed(&sig.inputs[1])?;
        let out = typed(&sig.inputs[2])?;

        route.doc += &inp.pat.to_token_stream().to_string();
        route.doc += "\noutput: ";
        route.doc += &out.pat.to_token_stream().to_string();

        route.inp = tup(inp, false)?;
        route.out = tup(out, true)?;

        Ok(route)
    }
}
