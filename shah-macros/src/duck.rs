use proc_macro::TokenStream;
use quote::quote;

pub(crate) fn duck(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
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

    quote! {
        #[automatically_derived]
        impl #generics #ci::db::pond::Duck for #ident #gnb {
            fn pond(&self) -> &Gene {
                &self.pond
            }
            fn pond_mut(&mut self) -> &mut Gene {
                &mut self.pond
            }
        }
    }
    .into()
}
