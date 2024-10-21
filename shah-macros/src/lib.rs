mod api;
mod enum_code;
mod model;
mod command;

use proc_macro::TokenStream;
use quote::quote;


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

#[proc_macro_derive(Entity)]
pub fn entity(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let ci = crate_ident();

    quote! {
        impl #ci::db::entity::Entity for #ident {
            fn gene(&self) -> &Gene {
                &self.gene
            }
            fn flags(&self) -> &u8 {
                &self.flags.as_binary()[0]
            }

            fn gene_mut(&mut self) -> &mut Gene {
                &mut self.gene
            }
            fn flags_mut(&mut self) -> &mut u8 {
                &mut self.flags.as_binary_mut()[0]
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
