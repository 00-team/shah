use proc_macro2::TokenStream as TokenStream2;
use quote_into::quote_into;
use syn::spanned::Spanned;

pub(crate) fn add_assign(item: syn::ItemStruct) -> syn::Result<TokenStream2> {
    let mut add = TokenStream2::new();
    let mut sub = TokenStream2::new();

    for f in item.fields {
        let Some(fname) = &f.ident else {
            return crate::err!(f.span(), "field with no name");
        };

        quote_into!(add += self.#fname = self.#fname.saturating_add(rhs.#fname););
        quote_into!(sub += self.#fname = self.#fname.saturating_sub(rhs.#fname););
    }

    let item_ident = &item.ident;

    Ok(quote::quote! {
        impl std::ops::AddAssign for #item_ident {
            fn add_assign(&mut self, rhs: Self) {
                #add
            }
        }

        impl std::ops::SubAssign for #item_ident {
            fn sub_assign(&mut self, rhs: Self) {
                #sub
            }
        }
    })
}
