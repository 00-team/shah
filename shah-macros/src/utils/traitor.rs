use crate::err;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use quote_into::quote_into;
use syn::spanned::Spanned;

pub struct TraitorField<'a> {
    name: &'static str,
    copy: bool,
    ty: &'a syn::Type,
    ident: syn::Ident,
}

impl<'a> TraitorField<'a> {
    pub fn new(name: &'static str, ty: &'a syn::Type, copy: bool) -> Self {
        Self {
            name,
            ty,
            copy,
            ident: syn::Ident::new(name, proc_macro2::Span::call_site()),
        }
    }
}

pub struct Traitor<'a, const LEN: usize> {
    tpath: syn::Path,
    attr_name: &'static str,
    fields: [TraitorField<'a>; LEN],
}

impl<'a, const LEN: usize> Traitor<'a, LEN> {
    pub fn new(
        attr_name: &'static str, tpath: syn::Path,
        fields: [TraitorField<'a>; LEN],
    ) -> Self {
        Self { attr_name, tpath, fields }
    }

    pub fn derive(mut self, inp: syn::DeriveInput) -> syn::Result<TokenStream> {
        let (impl_gnc, ty_gnc, where_gnc) = inp.generics.split_for_impl();
        let syn::Data::Struct(data) = &inp.data else {
            panic!(
                "{} trait is only ment for structs",
                self.tpath.to_token_stream()
            )
        };
        for f in &data.fields {
            for a in &f.attrs {
                let syn::Meta::List(ml) = &a.meta else {
                    continue;
                };
                if !ml.path.is_ident(self.attr_name) {
                    continue;
                }
                let kind = syn::parse::<syn::Ident>(ml.tokens.clone().into())?
                    .to_string();

                let ident = f.ident.clone().unwrap();

                let mut found = false;
                for tf in self.fields.iter_mut() {
                    if tf.name == kind {
                        found = true;
                        tf.ident = ident;
                        break;
                    }
                }

                if !found {
                    return err!(a.span(), "unknown value in attribute");
                }

                break;
            }
        }

        let mut s = TokenStream::new();

        for TraitorField { ident, name, copy, ty } in self.fields {
            let mutid = format_ident!("{name}_mut");
            quote_into! {s +=
                fn #name(&self) -> #{if !copy {quote_into!(s += &)}} #ty {
                    #{
                        if copy { quote_into!(s += *); }
                        else { quote_into!(s += &); }
                    }
                    self.#ident
                }

                fn #mutid(&mut self) -> &mut #ty {
                    &mut self.#ty
                }
            };
        }

        let ident = &inp.ident;
        let tpath = &self.tpath;
        Ok(quote::quote! {
            #[automatically_derived]
            impl #impl_gnc #tpath for #ident #ty_gnc #where_gnc {
                #s
            }
        })
    }
}
