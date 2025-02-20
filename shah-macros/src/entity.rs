use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use quote_into::quote_into;

pub(crate) fn entity(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let ci = crate::crate_ident();

    let mut flags_ident: Option<&syn::Ident> = None;
    let mut growth_ident: Option<&syn::Ident> = None;
    let mut gene_ident: Option<&syn::Ident> = None;

    let (impl_gnc, ty_gnc, where_gnc) = item.generics.split_for_impl();

    let syn::Data::Struct(data) = &item.data else {
        panic!("Entity Trait is only ment for structs")
    };
    for f in &data.fields {
        for a in &f.attrs {
            if let syn::Meta::List(ml) = &a.meta {
                if !ml.path.is_ident("entity") {
                    continue;
                }
                let kind = syn::parse::<syn::Ident>(ml.tokens.clone().into())
                    .unwrap()
                    .to_string();

                match kind.as_str() {
                    "flags" => flags_ident = f.ident.as_ref(),
                    "growth" => growth_ident = f.ident.as_ref(),
                    "gene" => gene_ident = f.ident.as_ref(),
                    _ => panic!("unknown entity kind: {kind}"),
                }
                break;
            }
        }
    }

    let Some(flags_ident) = flags_ident else {
        panic!("#[entity(flags)] is not set for field");
    };

    let Some(gene_ident) = gene_ident else {
        panic!("#[entity(gene)] is not set for field");
    };

    let Some(growth_ident) = growth_ident else {
        panic!("#[entity(growth)] is not set for field");
    };

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
        impl #impl_gnc #ci::db::entity::Entity for #ident #ty_gnc #where_gnc {
            fn gene(&self) -> &#ci::models::Gene {
                &self.#gene_ident
            }
            fn gene_mut(&mut self) -> &mut #ci::models::Gene {
                &mut self.#gene_ident
            }

            fn growth(&self) -> u64 {
                self.#growth_ident
            }
            fn growth_mut(&mut self) -> &mut u64 {
                &mut self.#growth_ident
            }

            #f
        }
    }
    .into()
}
