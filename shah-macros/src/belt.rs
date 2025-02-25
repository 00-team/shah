use crate::ident;
use proc_macro::TokenStream;
use quote::quote;

pub(crate) fn belt(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let ci = crate::crate_ident();

    let mut next_ident = ident!("next");
    let mut past_ident = ident!("past");
    let mut buckle_ident = ident!("buckle");

    let (impl_gnc, ty_gnc, where_gnc) = item.generics.split_for_impl();

    let syn::Data::Struct(data) = &item.data else {
        panic!("Belt Trait is only ment for structs")
    };
    for f in &data.fields {
        for a in &f.attrs {
            if let syn::Meta::List(ml) = &a.meta {
                if !ml.path.is_ident("belt") {
                    continue;
                }
                let kind = syn::parse::<syn::Ident>(ml.tokens.clone().into())
                    .unwrap()
                    .to_string();

                let ident = f.ident.clone().unwrap();

                match kind.as_str() {
                    "next" => next_ident = ident,
                    "past" => past_ident = ident,
                    "buckle" => buckle_ident = ident,
                    _ => panic!("unknown entity kind: {kind}"),
                }
                break;
            }
        }
    }

    quote! {
        impl #impl_gnc #ci::db::belt::Belt for #ident #ty_gnc #where_gnc {
            fn next(&self) -> &#ci::models::Gene {
                &self.#next_ident
            }
            fn next_mut(&mut self) -> &mut #ci::models::Gene {
                &mut self.#next_ident
            }

            fn past(&self) -> &#ci::models::Gene {
                &self.#past_ident
            }
            fn past_mut(&mut self) -> &mut #ci::models::Gene {
                &mut self.#past_ident
            }

            fn buckle(&self) -> &#ci::models::Gene {
                &self.#buckle_ident
            }
            fn buckle_mut(&mut self) -> &mut #ci::models::Gene {
                &mut self.#buckle_ident
            }
        }
    }
    .into()
}
