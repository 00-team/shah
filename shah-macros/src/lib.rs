mod api;
mod command;
mod duck;
mod entity;
mod enum_code;
mod enum_int;
mod model;
mod perms;
mod routes;

use proc_macro::TokenStream;

#[proc_macro_derive(Command)]
pub fn command(code: TokenStream) -> TokenStream {
    command::command(code)
}

#[proc_macro_attribute]
pub fn enum_code(args: TokenStream, code: TokenStream) -> TokenStream {
    enum_code::enum_code(args, code)
}

#[proc_macro_attribute]
pub fn enum_int(args: TokenStream, code: TokenStream) -> TokenStream {
    enum_int::enum_int(args, code)
}

#[proc_macro_attribute]
pub fn api(args: TokenStream, code: TokenStream) -> TokenStream {
    api::api(args, code)
}

#[proc_macro_attribute]
pub fn model(args: TokenStream, code: TokenStream) -> TokenStream {
    model::model(args, code)
}

#[proc_macro_derive(Entity, attributes(entity_flags))]
pub fn entity(code: TokenStream) -> TokenStream {
    entity::entity(code)
}

#[proc_macro_derive(Duck)]
pub fn duck(code: TokenStream) -> TokenStream {
    duck::duck(code)
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

    syn::Ident::new("shah", proc_macro2::Span::call_site())
}
