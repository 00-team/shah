mod api;
mod command;
mod enum_code;
mod model;

use core::panic;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use quote_into::quote_into;

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
                flags_ident = f.ident.as_ref();
            }
        }
    }

    if flags_ident.is_none() {
        panic!("#[entity_flags] is not set for");
    }

    const ENTITY_FLAGS: [&str; 3] = ["alive", "edited", "private"];
    let mut f = TokenStream2::new();
    for (i, flag) in ENTITY_FLAGS.iter().enumerate() {
        let fi = format_ident!("{flag}");
        let set = format_ident!("set_{flag}");
        quote_into! {f +=
            fn #fi(&self) -> bool {
                (self.#flags_ident & (1 << #i)) == (1 << #i)
            }

            fn #set(&mut self, #fi: bool) -> &mut Self {
                if #fi {
                    self.#flags_ident |= (1 << #i);
                } else {
                    self.#flags_ident &= !(1 << #i);
                }
                self
            }
        };
    }

    quote! {
        impl #ci::db::entity::Entity for #ident {
            fn gene(&self) -> &Gene {
                &self.gene
            }
            fn gene_mut(&mut self) -> &mut Gene {
                &mut self.gene
            }

            #f
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
