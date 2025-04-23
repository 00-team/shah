macro_rules! ez_trait {
    (
    $fn_name: ident,
    $db_path: path,
    $(
        $f_name: literal,
        $fty: path,
        $f_var: ident,
        $fget: ident,
        $fmut: ident;
    )*
    ) => {
pub(crate) fn $fn_name (code: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = syn::parse_macro_input!(code as syn::DeriveInput);
    let ident = &item.ident;
    let ci = crate::crate_ident();

    $( let mut $f_var = crate::ident!($f_name); )*

    let (impl_gnc, ty_gnc, where_gnc) = item.generics.split_for_impl();

    let syn::Data::Struct(data) = &item.data else {
        panic!(concat!(stringify!($db_path), " trait is only ment for structs"))
    };
    for f in &data.fields {
        for a in &f.attrs {
            if let syn::Meta::List(ml) = &a.meta {
                if !ml.path.is_ident(stringify!($fn_name)) {
                    continue;
                }
                let kind = syn::parse::<syn::Ident>(ml.tokens.clone().into())
                    .unwrap()
                    .to_string();

                let ident = f.ident.clone().unwrap();

                match kind.as_str() {
                    $( $f_name => $f_var = ident, )*
                    _ => panic!("unknown entity kind: {kind}"),
                }
                break;
            }
        }
    }

    quote::quote! {
        #[automatically_derived]
        impl #impl_gnc #ci::db::$db_path for #ident #ty_gnc #where_gnc {
            $(
                fn $fget(&self) -> &#ci::$fty {
                    &self.#$f_var
                }

                fn $fmut(&mut self) -> &mut #ci::$fty {
                    &mut self.#$f_var
                }
            )*
        }
    }
    .into()
}

    };
}

pub(crate) use ez_trait;
