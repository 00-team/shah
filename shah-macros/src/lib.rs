mod api;
mod command;
mod enum_code;
mod model;

use core::panic;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};

#[proc_macro_derive(Command)]
pub fn command(code: TokenStream) -> TokenStream {
    command::command(code)
}

#[proc_macro_attribute]
pub fn enum_code(args: TokenStream, code: TokenStream) -> TokenStream {
    enum_code::enum_code(args, code)
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
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let mut flags_ident: Option<&syn::Ident> = None;
    let ci = crate_ident();

    let syn::Data::Struct(data) = &item.data else {
        panic!("Entity Trait is only ment for structs")
    };
    for f in &data.fields {
        for a in &f.attrs {
            if a.meta.to_token_stream().to_string() == "entity_flags" {
                if flags_ident.is_some() {
                    panic!("only one entity_flags field is allowed")
                }
                let ty = f.ty.to_token_stream().to_string();
                if ty != "u8" {
                    panic!("#[entity_flags] must be u8: {ty} != u8")
                }
                flags_ident = f.ident.as_ref();
            }
        }
    }

    if flags_ident.is_none() {
        panic!("#[entity_flags] is not set for");
    }

    quote! {
        impl #ci::db::entity::Entity for #ident {
            fn gene(&self) -> &Gene {
                &self.gene
            }
            fn gene_mut(&mut self) -> &mut Gene {
                &mut self.gene
            }

            fn flags(&self) -> &u8 {
                &self.#flags_ident
            }
            
            fn flags_mut(&mut self) -> &mut u8 {
                &mut self.#flags_ident
            }
        }
    }
    .into()
}

fn crate_ident() -> syn::Ident {
    // let found_crate = crate_name("shah").unwrap();
    // let name = match &found_crate {
    //     FoundCrate::Itself => "shah",
    //     FoundCrate::Name(name) => name,
    // };

    syn::Ident::new("shah", proc_macro2::Span::call_site())
}
