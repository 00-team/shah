use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use quote_into::quote_into;

pub(crate) fn entity(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let mut flags_ident: Option<&syn::Ident> = None;
    let ci = crate::crate_ident();

    let generics = &item.generics;
    let mut gnb = item.generics.clone();
    for p in gnb.params.iter_mut() {
        match p {
            syn::GenericParam::Type(t) => t.bounds.clear(),
            _ => {
                panic!("invalid generic param")
            }
        }
    }

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
        let get = format_ident!("is_{flag}");
        let set = format_ident!("set_{flag}");
        quote_into! {f +=
            fn #get(&self) -> bool {
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
        impl #generics #ci::db::entity::Entity for #ident #gnb {
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
