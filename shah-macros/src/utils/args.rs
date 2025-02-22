#[allow(unused_macros)]
macro_rules! args_parse {
    (
        @derive_only
        $( #[$attr:meta] )*
        $pub:vis
        struct $StructName:ident {
            $(
                $( #[$field_attr:meta] )*
                $field_pub:vis
                $field_name:ident : $FieldTy:ty
            ),* $(,)?
        }
    ) => {
        impl ::syn::parse::Parse for $StructName {
            fn parse(input: ::syn::parse::ParseStream<'_>) -> ::syn::Result<Self> {
                mod kw { $( ::syn::custom_keyword!( $field_name ); )* }
                use ::core::ops::Not as _;

                $( let mut $field_name = ::core::option::Option::None::< $FieldTy >; )*
                while input.is_empty().not() {
                    let lookahead = input.lookahead1();
                    match () {
                        $(
                        _case if lookahead.peek(kw::$field_name) => {
                            let span = input.parse::<kw::$field_name>().unwrap().span;
                            let _: ::syn::Token![ = ] = input.parse()?;
                            let prev = $field_name.replace(input.parse()?);
                            if prev.is_some() {
                                return ::syn::Result::Err(::syn::Error::new(span, "Duplicate key"));
                            }
                        },
                        )*
                        _default => return ::syn::Result::Err(lookahead.error()),
                    }
                    let _: ::core::option::Option<::syn::Token![ , ]> = input.parse()?;
                }
                Ok(Self {
                    $(
                        $field_name: $field_name.ok_or_else(|| ::syn::Error::new(
                            ::proc_macro2::Span::call_site(),
                            ::core::concat!("Missing key `", ::core::stringify!($field_name), "`"),
                        ))?,
                    )*
                })
            }
        }
    };

    ( $( #[$attr:meta] )* $pub:vis struct $($rest:tt)* ) => {
        $( #[$attr] )* $pub struct $($rest)*
        args_parse! { @derive_only  $( #[$attr] )* $pub struct $($rest)* }
    }
}

#[allow(unused_imports)]
pub(crate) use args_parse;
