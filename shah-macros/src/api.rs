use crate::err;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use quote_into::quote_into;
use syn::{punctuated::Punctuated, spanned::Spanned};

type Args = Punctuated<syn::MetaNameValue, syn::token::Comma>;

pub(crate) fn api(
    args: Args, mut item: syn::ItemMod,
) -> syn::Result<TokenStream2> {
    let item_span = item.span();
    let Some((_, content)) = &mut item.content else {
        return err!(item_span, "mod is empty");
    };

    let user_mod = &item.ident;
    if user_mod == "api" {
        return err!(user_mod.span(), "mod api is a reserved name");
    }

    if content.is_empty() {
        return err!(item_span, "mod is empty");
    }

    let mut s = TokenStream2::new();

    let ApiArgs { api_scope, user_error } = parse_args(args)?;
    let ci = crate::crate_ident();
    let mut use_list = TokenStream2::new();
    let mut user_funcs = Vec::<&syn::ItemFn>::new();
    // let mut user_client = TokenStream2::new();
    let bin = quote! { #ci::models::Binary };

    for item in content.iter() {
        match &item {
            syn::Item::Fn(f) => {
                if f.attrs.iter().any(|a| a.path().is_ident("client")) {
                    return err!(f.span(), "#[client] is deprecated");
                    // f.attrs.clear();
                    // quote_into!(user_client += #f);
                }

                match f.vis {
                    syn::Visibility::Inherited => {}
                    _ => user_funcs.push(f),
                }
            }
            syn::Item::Use(u) => quote_into!(use_list += #u),
            // syn::Item::Use(u) => {
            //     return err!(
            //         u.span(),
            //         "put all of your `use`'s in the parent mod"
            //     )
            // }
            _ => return err!(item.span(), "only fn's and use's are valid"),
        }
    }

    if user_funcs.is_empty() {
        return err!(
            item_span,
            "you have no api function. you mut at least have one ",
            "function with visibility of pub(super) or higher"
        );
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

    let mut a = TokenStream2::new();
    // user_funcs.iter().for_each(|f| quote_into!(a += #f));

    for Route { api_ident, ident, inp, out, ret, .. } in routes.iter() {
        let mut output_var = TokenStream2::new();
        for (vid, t) in out.iter() {
            quote_into! {output_var +=
                let (#vid, out) = out.split_at_mut(<#t as #bin>::S);
                let #vid = <#t as #bin>::from_binary_mut(#vid);
            };
        }

        let mut cos = quote!(0);
        for (_, t) in out.iter() {
            quote_into!(cos += + <#t as #bin>::S);
        }

        let mut cis = quote!(0);
        for (_, t) in inp.iter() {
            quote_into!(cis += + <#t as #bin>::S);
        }

        let mut bf = TokenStream2::new();
        quote_into! {bf += 0};
        let mut input_var = TokenStream2::new();
        for (_, t) in inp.iter() {
            quote_into! {input_var +=
                <#t as #bin>::from_binary(&inp[#bf..#bf + <#t as #bin>::S]),
            };
            quote_into! {bf += + <#t as #bin>::S};
        }

        let retval = if *ret { quote!(Ok(res)) } else { quote!(Ok(#cos)) };

        let x = inp.iter().map(|(_, t)| t.into_token_stream().to_string());
        let cis_ty = x.collect::<Vec<_>>().join(", ");
        let cis_err = format!(
            "input size exceeds the maximum at: {ident}(inp: {cis_ty})"
        );

        let x = out.iter().map(|(_, t)| t.into_token_stream().to_string());
        let cos_ty = x.collect::<Vec<_>>().join(", ");
        let cos_err = format!(
            "output size exceeds the maximum at: {ident}(out: {cos_ty})"
        );

        quote_into! {a +=
            const _: () = {
                assert!(#cis < #ci::ORDER_BODY_SIZE, #cis_err);
                assert!(#cos < #ci::REPLY_BODY_SIZE, #cos_err);
            };

            #[allow(dead_code)]
            pub(crate) fn #api_ident(state: &mut #state, inp: &[u8], out: &mut [u8]) -> Result<usize, #ci::ErrorCode> {
                let input = (#input_var);
                #output_var
                let output = (#{
                    out.iter().for_each(|(vid, _)| quote_into!(a += #vid,))
                });
                let res = #user_mod :: #ident(state, input, output)?;
                #retval
            }
        };
    }

    let mut routes_api = TokenStream2::new();
    for Route { api_ident, ident, inp, out, .. } in routes.iter() {
        let mut is = TokenStream2::new();
        quote_into!(is += 0);
        inp.iter().for_each(|(_, t)| quote_into!(is += + <#t as #bin>::S));

        let mut cos = quote!(0);
        out.iter().for_each(|(_, t)| quote_into!(cos += + <#t as #bin>::S));

        let name = ident.to_string();
        quote_into! {routes_api +=
            #ci::models::Api::<#state> {
                name: #name,
                caller: #api_ident,
                input_size: #is + (8 - (#is) % 8),
                max_output_size: #cos,
            },
        }
    }

    let mut c = TokenStream2::new();
    for (rdx, Route { ident, inp, out, doc, .. }) in routes.iter().enumerate() {
        let mut is = TokenStream2::new();
        quote_into!(is += 0);
        inp.iter().for_each(|(_, t)| quote_into!(is += + <#t as #bin>::S));

        let mut bf = TokenStream2::new();
        quote_into! {bf += 0};
        let mut inp_res = TokenStream2::new();
        for (i, t) in inp.iter() {
            quote_into! {inp_res +=
                __order_body[#bf..#bf + <#t as #bin>::S]
                .clone_from_slice(<#t as #bin>::as_binary(#i));
            };
            quote_into! {bf += + <#t as #bin>::S};
        }

        let mut fn_inp = TokenStream2::new();
        inp.iter().for_each(|(i, t)| quote_into!(fn_inp += #i: &#t, ));

        let (out_ty, out_res) = if out.is_empty() {
            (quote!(()), quote!(()))
        } else if out.len() == 1 {
            let t = &out[0].1;

            (
                quote!(#t),
                quote!(<#t as #bin>::from_binary(&__reply.body[..<#t as #bin>::S]).clone()),
            )
        } else {
            let mut ot = TokenStream2::new();
            quote_into! {ot += (
                #{out.iter().for_each(|(_, t)| quote_into!(ot += #t,))}
            )};

            let mut bf = quote!(0);
            let mut or = TokenStream2::new();
            for (_, t) in out.iter() {
                quote_into! {or +=
                    <#t as #bin>::from_binary(
                        &__reply.body[#bf..#bf + <#t as #bin>::S]
                    ).clone(),
                };
                quote_into! {bf += + <#t as #bin>::S};
            }

            (ot, quote! { ( #or ) })
        };

        quote_into! {c +=
            #[doc = #doc]
            pub fn #ident(
                taker: &#ci::Taker, #fn_inp
            ) -> Result<#out_ty, #ci::ClientError<#user_error>> {
                let mut __order = [0u8; #is + (8 - (#is) % 8) + <#ci::models::OrderHead as #bin>::S];
                let (__order_head, __order_body) = __order.split_at_mut(<#ci::models::OrderHead as #bin>::S);
                let __order_head = <#ci::models::OrderHead as #bin>::from_binary_mut(__order_head);
                __order_head.scope = ( #api_scope ) as u16;
                __order_head.route = ( #rdx ) as u16;
                __order_head.size = ( #is ) as u32;

                #inp_res

                let __reply = taker.take(&mut __order)?;
                // let reply_head = taker.reply_head();
                // let reply_body = taker.reply_body(reply_head.size as usize);
                Ok(#out_res)
            }
        }
    }

    let mut at = TokenStream2::new();
    for Route { ident, inp, out, .. } in routes.iter() {
        let mut is = TokenStream2::new();
        quote_into!(is += 0);
        inp.iter().for_each(|(_, t)| quote_into!(is += + <#t as #bin>::S));

        let mut bf = TokenStream2::new();
        quote_into! {bf += 0};
        let mut input_vars = TokenStream2::new();
        for (_, t) in inp.iter() {
            let ts = t.to_token_stream().to_string();
            quote_into! {input_vars +=
                println!("{}::from_binary: {} + {} / {}", #ts, #bf, <#t as #bin>::S, __order_body.len());
                // let _ = <#t as #bin>::as_binary(&#t::default());
                let _ = <#t as #bin>::from_binary(&__order_body[#bf..#bf + <#t as #bin>::S]);
            };
            quote_into! {bf += + <#t as #bin>::S};
        }

        let mut bf = quote!(0);
        let mut output_res = TokenStream2::new();
        for (_, t) in out.iter() {
            let ts = t.to_token_stream().to_string();
            quote_into! {output_res +=
                println!("{}::from_binary: {} + {} / {}", #ts, #bf, <#t as #bin>::S, __reply.body.len());
                let _ = <#t as #bin>::from_binary(&__reply.body[#bf..#bf + <#t as #bin>::S]);
            };
            quote_into! {bf += + <#t as #bin>::S};
        }

        quote_into! {at +=
            #[test]
            pub fn #ident() {
                println!("\n");

                let mut __order = [0u8; #is + (8 - (#is) % 8) + <#ci::models::OrderHead as #bin>::S];
                let (__order_head, __order_body) = __order.split_at_mut(<#ci::models::OrderHead as #bin>::S);
                let __order_head = <#ci::models::OrderHead as #bin>::from_binary_mut(__order_head);
                __order_head.size = ( #is ) as u32;

                println!("inputs");
                #input_vars

                let __reply = shah::models::Reply::default();
                println!("outputs");
                #output_res

            }
        }
    }

    let routes_len = routes.len();

    quote_into! {s +=
        #item

        pub(crate) mod gen_api {

            #![allow(unused_imports)]
            #use_list

            #a

            pub(crate) const ROUTES: [#ci::models::Api<#state>; #routes_len] = [#routes_api];
            pub(crate) const FILE: &str = file!();
            pub(crate) const SCOPE: usize = #api_scope;
        }

        pub mod client {
            #![allow(unused_imports)]
            #use_list

            #c
        }

        #[cfg(test)]
        mod api_tests {
            #![allow(unused_imports)]
            #use_list

            #at
        }
    };

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

fn returns_output_size(span: Span, rt: &syn::ReturnType) -> syn::Result<bool> {
    macro_rules! e {
        () => {
            err!(
                rt.span(),
                "return type of an api must be Result<(), ErrorCode> ",
                "or Result<usize, ErrorCode>"
            )
        };
    }

    let syn::ReturnType::Type(_, t) = rt else {
        return err!(span, "no return type has been specified");
    };

    let syn::Type::Path(p) = &(**t) else { return e!() };
    let args = &p.path.segments[0].arguments;

    let syn::PathArguments::AngleBracketed(a) = args else { return e!() };
    let syn::GenericArgument::Type(t) = &a.args[0] else { return e!() };

    if let syn::Type::Tuple(tp) = t
        && tp.elems.is_empty()
    {
        return Ok(false);
    }

    if let syn::Type::Path(p) = t
        && p.to_token_stream().to_string() == "usize"
    {
        return Ok(true);
    }

    e!()
}

type RouteArgs = Vec<(syn::Ident, syn::Type)>;

#[derive(Debug)]
struct Route {
    state: syn::Type,
    ident: syn::Ident,
    api_ident: syn::Ident,
    inp: RouteArgs,
    out: RouteArgs,
    ret: bool,
    doc: String,
}

fn arr_name(ty: &syn::Type, d: usize) -> String {
    match ty {
        syn::Type::Path(p) => {
            let Some(s) = p.path.segments.last() else {
                return "arr".to_string();
            };

            s.ident.to_string()
        }
        syn::Type::Array(a) => {
            if d > 3 {
                return "arr".to_string();
            }
            arr_name(&a.elem, d + 1)
        }
        _ => "arr".to_string(),
    }
}

impl Route {
    fn args(fnarg: &syn::PatType, mm: bool) -> syn::Result<RouteArgs> {
        let syn::Type::Tuple(tt) = &(*fnarg.ty) else {
            return err!(fnarg.span(), "input and output types must be tuple");
        };
        let mut names = Vec::<&syn::Ident>::with_capacity(tt.elems.len());
        let mut res = RouteArgs::with_capacity(tt.elems.len());

        let prefix = if mm { "o" } else { "i" };
        let mut has_names = false;

        if !mm && let syn::Pat::Tuple(pt) = &(*fnarg.pat) {
            if pt.elems.len() != tt.elems.len() {
                return err!(
                    pt.span(),
                    "you must specify a name for all the types"
                );
            }

            for e in pt.elems.iter() {
                let syn::Pat::Ident(ei) = e else {
                    return err!(e.span(), "all names must be idents");
                };
                if ei.ident == "taker" {
                    return err!(ei.span(), "taker is a reserved ident");
                }
                names.push(&ei.ident);
            }
            has_names = true
        }

        for (idx, t) in tt.elems.iter().enumerate() {
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

            let tyn = match &(*tr.elem) {
                syn::Type::Path(p) => {
                    p.path.segments.last().unwrap().ident.to_string()
                }
                syn::Type::Array(a) => arr_name(&a.elem, 0),
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
            .to_lowercase();

            if has_names {
                res.push((names[idx].clone(), *tr.elem.clone()));
            } else {
                res.push((
                    format_ident!("{tyn}_{prefix}{idx}"),
                    *tr.elem.clone(),
                ));
            }
        }

        Ok(res)
    }

    fn from_signature(sig: &syn::Signature) -> syn::Result<Self> {
        let mut route = Route {
            state: syn::Type::Never(syn::TypeNever {
                bang_token: Default::default(),
            }),
            ident: sig.ident.clone(),
            api_ident: format_ident!("{}_api", sig.ident),
            inp: Default::default(),
            out: Default::default(),
            ret: returns_output_size(sig.span(), &sig.output)?,
            doc: "input: ".to_string(),
        };

        if sig.inputs.len() != 3 {
            return err!(
                sig.span(),
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

        let inp = typed(&sig.inputs[1])?;
        let out = typed(&sig.inputs[2])?;

        route.doc += &inp.pat.to_token_stream().to_string();
        route.doc += "\noutput: ";
        route.doc += &out.pat.to_token_stream().to_string();

        route.inp = Self::args(inp, false)?;
        route.out = Self::args(out, true)?;

        Ok(route)
    }
}
