use syn::parse_quote;
use utils::traitor::{Traitor, TraitorField};

mod api;
mod entity;
mod flags;
mod model;
mod routes;
mod schema;
mod utils;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn api(args: TokenStream, code: TokenStream) -> TokenStream {
    type Args = syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]>;
    let item = syn::parse_macro_input!(code as syn::ItemMod);
    let attrs = syn::parse_macro_input!(args with Args::parse_terminated);

    api::api(attrs, item).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_attribute]
pub fn model(_args: TokenStream, code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemStruct);
    model::model(item).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_attribute]
pub fn flags(args: TokenStream, code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemStruct);
    flags::flags(args, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Entity, attributes(entity))]
/// Derive macro generating an impl of the trait `Entity`.
///
/// You can use `#[entity(gene)]`, `#[entity(flags)]` and `#[entity(growth)]`
/// to set custom fields for these methods.
pub fn entity(code: TokenStream) -> TokenStream {
    entity::entity(code)
}

#[proc_macro_derive(Belt, attributes(belt))]
/// Derive macro generating an impl of the trait `Belt`.
/// You can use `#[belt(next)]`, `#[belt(past)]` and `#[belt(buckle)]`
/// to set custom fields for these methods.
pub fn belt(code: TokenStream) -> TokenStream {
    let ci = crate_ident();
    let inp = syn::parse_macro_input!(code as syn::DeriveInput);

    let gene = parse_quote!(#ci::models::Gene);

    let tr = Traitor::new(
        "belt",
        parse_quote!(#ci::db::belt::Belt),
        [
            TraitorField::new("next", &gene, false),
            TraitorField::new("past", &gene, false),
            TraitorField::new("buckle", &gene, false),
        ],
    );
    tr.derive(inp).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(Buckle, attributes(buckle))]
pub fn buckle(code: TokenStream) -> TokenStream {
    let ci = crate_ident();
    let inp = syn::parse_macro_input!(code as syn::DeriveInput);

    let gene = parse_quote!(#ci::models::Gene);
    let pu64 = parse_quote!(u64);

    let tr = Traitor::new(
        "buckle",
        parse_quote!(#ci::db::belt::Buckle),
        [
            TraitorField::new("head", &gene, false),
            TraitorField::new("tail", &gene, false),
            TraitorField::new("belt_count", &pu64, true),
        ],
    );
    tr.derive(inp).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(Duck, attributes(duck))]
pub fn duck(code: TokenStream) -> TokenStream {
    let ci = crate_ident();
    let inp = syn::parse_macro_input!(code as syn::DeriveInput);
    let gene = parse_quote!(#ci::models::Gene);
    let tr = Traitor::new(
        "duck",
        parse_quote!(#ci::db::pond::Duck),
        [TraitorField::new("pond", &gene, false)],
    );
    tr.derive(inp).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(Pond, attributes(pond))]
pub fn pond(code: TokenStream) -> TokenStream {
    let ci = crate_ident();
    let inp = syn::parse_macro_input!(code as syn::DeriveInput);

    let gene = parse_quote!(#ci::models::Gene);
    let pru8 = parse_quote!(u8);
    let gene_id = parse_quote!(#ci::models::GeneId);

    let tr = Traitor::new(
        "pond",
        parse_quote!(#ci::db::pond::Pond),
        [
            TraitorField::new("next", &gene, false),
            TraitorField::new("past", &gene, false),
            TraitorField::new("origin", &gene, false),
            TraitorField::new("stack", &gene_id, true),
            TraitorField::new("alive", &pru8, true),
            TraitorField::new("empty", &pru8, true),
        ],
    );
    tr.derive(inp).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(Origin, attributes(origin))]
pub fn origin(code: TokenStream) -> TokenStream {
    let ci = crate_ident();
    let inp = syn::parse_macro_input!(code as syn::DeriveInput);

    let gene = parse_quote!(#ci::models::Gene);
    let pu64 = parse_quote!(u64);

    let tr = Traitor::new(
        "origin",
        parse_quote!(#ci::db::pond::Origin),
        [
            TraitorField::new("head", &gene, false),
            TraitorField::new("tail", &gene, false),
            TraitorField::new("pond_count", &pu64, true),
            TraitorField::new("item_count", &pu64, true),
        ],
    );
    tr.derive(inp).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(ShahSchema)]
pub fn schema(code: TokenStream) -> TokenStream {
    schema::schema(code)
}

#[proc_macro]
pub fn routes(code: TokenStream) -> TokenStream {
    routes::routes(code)
}

fn crate_ident() -> syn::Ident {
    // let found_crate = crate_name("shah").unwrap();
    // let name = match &found_crate {
    //     FoundCrate::Itself => "shah",
    //     FoundCrate::Name(name) => name,
    // };
    ident!("shah")
}

macro_rules! ident {
    ($name:literal) => {
        syn::Ident::new($name, proc_macro2::Span::call_site())
    };
    ($name:expr) => {
        syn::Ident::new($name, proc_macro2::Span::call_site())
    };
}
pub(crate) use ident;

macro_rules! err {
    ($span:expr, $($msg:literal),*) => {
        Err(syn::Error::new($span, concat!( $($msg),* )))
    };
    ($span:expr, $msg:expr) => {
        Err(syn::Error::new($span, $msg))
    };
}

pub(crate) use err;
