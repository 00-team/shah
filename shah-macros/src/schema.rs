use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote_into::quote_into;

// use crate::utils::args::args_parse;

pub(crate) fn schema(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let ci = crate::crate_ident();
    let model_name = ident.to_string();
    let mut s = TokenStream2::new();

    let (impl_gnc, ty_gnc, where_gnc) = item.generics.split_for_impl();

    let data = match item.data {
        syn::Data::Struct(ds) => ds,
        _ => panic!("ShahSchema derive macro is only for structs"),
    };

    fn quote_schema(
        args: &FieldArgs, ty: &syn::Type, s: &mut TokenStream2, ci: &syn::Ident,
    ) {
        match ty {
            syn::Type::Array(syn::TypeArray { len, elem, .. }) => {
                let is_str = args.is_str;
                quote_into! {s += #ci::models::Schema::Array {
                    is_str: #is_str,
                    length: ( #len ) as u64,
                    kind: Box::new(#{quote_schema(args, elem, s, ci)}),
                }}
            }
            syn::Type::Tuple(t) => {
                quote_into! {s += #ci::models::Schema::Tuple(vec![#{
                    t.elems.iter().for_each(|e| quote_schema(args, e, s, ci))
                }])}
            }
            syn::Type::Path(t) => {
                quote_into! {s += <#t as #ci::models::ShahSchema>::shah_schema()}
            }
            _ => panic!("unknwon schema type: {ty:?} for ShahSchema"),
        }
    }

    fn fields(s: &mut TokenStream2, fields: &syn::Fields, ci: &syn::Ident) {
        for f in fields.iter() {
            let args = FieldArgs::from_attrs(&f.attrs).unwrap();
            let ident = f.ident.clone().unwrap().to_string();
            quote_into! {s += (
                String::from(#ident),
                #{quote_schema(&args, &f.ty, s, ci)}
            ),};
        }
    }

    quote_into! {s +=
        #[automatically_derived]
        impl #impl_gnc #ci::models::ShahSchema for #ident #ty_gnc #where_gnc {
            fn shah_schema() -> #ci::models::Schema {
                #ci::models::Schema::Model(#ci::models::SchemaModel {
                    name: String::from(#model_name),
                    size: core::mem::size_of::<Self>() as u64,
                    fields: vec![#{fields(s, &data.fields, &ci)}]
                })
            }
        }
    };

    s.into()
}

// args_parse! {
//     #[derive(Debug, Default)]
//     struct FieldArgs {
//         kind: Option<syn::Ident>,
//     }
// }

#[derive(Debug, Default)]
struct FieldArgs {
    is_str: bool,
}

impl FieldArgs {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut args = Self::default();
        for a in attrs.iter() {
            if a.path().is_ident("str") {
                args.is_str = true;
            }
            // if let syn::Meta::List(ml) = &a.meta {
            //     if !ml.path.is_ident("shah_schema") {
            //         continue;
            //     }
            //     let na: FieldArgs = syn::parse(ml.tokens.clone().into())?;
            //
            //     if let Some(kind) = na.kind {
            //         if args.kind.replace(kind).is_some() {
            //             panic!("duplicate kind")
            //         }
            //     }
            // }
        }

        Ok(args)
    }

    // fn is_str(&self) -> bool {
    //     matches!(&self.kind, Some(k) if k == "str")
    // }
}
