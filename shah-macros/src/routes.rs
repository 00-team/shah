// let routes = {
//     let mut routes: [&[ExampleApi]; 3] = Default::default();
//     assert_eq!(routes[phone::api::SCOPE].len(), 0);
//     routes[phone::api::SCOPE] = phone::api::ROUTES.as_slice();
//     assert_eq!(routes[phone::api::SCOPE].len(), 0);
//     routes[user::api::SCOPE] = user::api::ROUTES.as_slice();
//     routes[detail::api::SCOPE] = detail::api::ROUTES.as_slice();
//
//     routes
// };

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote_into::quote_into;

type IdentList = syn::punctuated::Punctuated<syn::Path, syn::Token![,]>;

pub fn routes(code: TokenStream) -> TokenStream {
    let ci = crate::crate_ident();
    let paths = syn::parse_macro_input!(code with IdentList::parse_terminated);
    if paths.len() < 2 {
        panic!("routes reqiure it least two params");
    }

    let len = paths.len() - 1;
    let mut paths = paths.iter();
    let state = paths.next().unwrap();

    let mut s = TokenStream2::new();
    quote_into! {s +=
        let mut routes: [Option<#ci::models::Scope<#state>>; #len] = [const {None}; #len];
    };
    for p in paths {
        let si = p.to_token_stream().to_string();
        quote_into! {s +=
            if let Some(scope) = &routes[#p::api::SCOPE] {
                panic!(
                    "scope: \x1b[32m{}\x1b[m is already is use by: \x1b[93m{}\x1b[m and cannot be used for: \x1b[93m{}\x1b[m",
                    #p::api::SCOPE, scope.name, #si
                );
            }
            routes[#p::api::SCOPE] = Some(#ci::models::Scope::<#state> {
                routes: #p::api::ROUTES.as_slice(),
                scope: #p::api::SCOPE,
                name: #si,
            });
        };
    }

    let mut p = TokenStream2::new();
    quote_into! {p += {
        #s

        routes.map(|s| s.unwrap())
    }};

    p.into()
}
