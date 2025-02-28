mod api;
mod belt;
mod command;
mod duck;
mod entity;
mod enum_code;
mod enum_int;
mod legacy;
mod model;
mod perms;
mod routes;
mod schema;
mod utils;

use proc_macro::TokenStream;

#[proc_macro_derive(Command)]
pub fn command(code: TokenStream) -> TokenStream {
    command::command(code)
}

#[proc_macro_derive(EnumCode, attributes(enum_code))]
pub fn enum_code(code: TokenStream) -> TokenStream {
    enum_code::enum_code(code)
}

#[proc_macro_attribute]
pub fn legacy(_args: TokenStream, code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemMod);
    legacy::legacy(item).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_attribute]
pub fn enum_int(args: TokenStream, code: TokenStream) -> TokenStream {
    enum_int::enum_int(args, code)
}

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
    belt::belt(code)
}

#[proc_macro_derive(Duck)]
pub fn duck(code: TokenStream) -> TokenStream {
    duck::duck(code)
}

#[proc_macro_derive(ShahSchema)]
pub fn schema(code: TokenStream) -> TokenStream {
    schema::schema(code)
}

#[proc_macro]
pub fn routes(code: TokenStream) -> TokenStream {
    routes::routes(code)
}

#[proc_macro]
pub fn perms(code: TokenStream) -> TokenStream {
    perms::perms(code)
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
}
pub(crate) use ident;

macro_rules! err {
    ($span:expr, $($msg:literal),*) => {
        Err(syn::Error::new(
            $span,
            concat!( $($msg),* )
        ))
    };
}

pub(crate) use err;
