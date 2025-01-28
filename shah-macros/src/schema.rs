use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote_into::quote_into;

pub(crate) fn schema(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let ci = crate::crate_ident();
    let model_name = ident.to_string();
    let mut s = TokenStream2::new();

    let data = match item.data {
        syn::Data::Struct(ds) => ds,
        _ => panic!("ShahSchema derive macro is only for structs"),
    };

    fn quote_schema(ty: &syn::Type, s: &mut TokenStream2, ci: &syn::Ident) {
        match ty {
            syn::Type::Array(syn::TypeArray { len, elem, .. }) => {
                quote_into! {s += #ci::schema::Schema::Array {
                    length: #len,
                    kind: Box::new(#{quote_schema(elem, s, ci)}),
                },}
            }
            syn::Type::Path(t) => {
                quote_into! {s += <#t as #ci::schema::ShahSchema>::shah_schema(),}
            }
            _ => panic!("unknwon type: {ty:?} for ShahSchema"),
        }
    }

    quote_into! {s +=
        impl #ci::schema::ShahSchema for #ident {
            fn shah_schema() -> #ci::schema::Schema {
                #ci::schema::Schema::Model {
                    name: String::from(#model_name),
                    size: core::mem::size_of::<#ident>() as u64,
                    fields: vec![#{
                        data.fields.iter().for_each(|f| quote_schema(&f.ty, s, &ci))
                    }]
                }
            }
        }
    };

    s.into()
}
