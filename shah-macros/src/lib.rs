mod api;
mod belt;
mod command;
mod entity;
mod enum_code;
mod enum_int;
mod legacy;
mod model;
mod perms;
mod pond;
mod routes;
mod schema;
mod utils;

use proc_macro::TokenStream;

/// Example:
/// ```ignore
/// #[derive(Debug, Default, shah::Command)]
/// enum MyCommands {
///     #[default]
///     Help,
///     Run,
///     Master { gene: shah::models::Gene }
/// }
/// ```
#[proc_macro_derive(Command)]
pub fn command(code: TokenStream) -> TokenStream {
    command::command(code)
}

/// Example:
/// ```ignore
/// #[derive(Debug, shah::EnumCode)]
/// #[enum_code(u8)]
/// pub enum Schema {
///     Model(Model), // 0u8
///     Array { len: usize }, // 1u8
///     U8,  // 2u8
///     Gene // 3u8
/// }
/// ```
#[proc_macro_derive(EnumCode, attributes(enum_code))]
pub fn enum_code(code: TokenStream) -> TokenStream {
    enum_code::enum_code(code)
}

/// Example:
/// ```ignore
/// #[shah::legacy]
/// mod items {
///
///     /// the derive macros from **Base** are set for all children
///     #[derive(Debug, Serialize, Deserialize, ToSchema)]
///     pub struct Base {
///         gene: Gene,
///         is_alive: bool,
///     }
///
///     impl From<&Review> for Base {
///         fn from(value: &Review) -> Self {
///             Self {
///                 gene: value.gene,
///                 is_alive: value.is_alive(),
///             }
///         }
///     }
///
///     // child 1
///     pub struct ReviewInfo {
///         // gene: Gene,
///         // is_alive: bool,
///         user: Gene,
///     }
///
///     impl From<&Review> for ReviewInfo {
///         fn from(value: &Review) -> Self {
///             Self { user: value.user }
///         }
///     }
///
///     // child 2
///     pub struct EateryReviewInfo {
///         // gene: Gene,
///         // is_alive: bool,
///         user: Option<EateryReviewUserInfo>,
///     }
///
///     impl From<&Review> for EateryReviewInfo {
///         fn from(review: &Review) -> Self {
///             Self { user: None }
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn legacy(_args: TokenStream, code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemMod);
    legacy::legacy(item).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// enum_ini is a two way conversion enum `<->` u16
/// default **start** is `0` and default **ty** is `u8`
/// Example:
/// ```ignore
/// #[shah::enum_int(u16)]
/// #[derive(Debug, Default, Clone, Copy)]
/// pub enum ExampleError {
///     #[default]
///     Unknown = 0,
///     UserNotFound,
///     BadPhone,
///     BadStr,
/// }
/// ```
#[proc_macro_attribute]
pub fn enum_int(args: TokenStream, code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemEnum);
    enum_int::enum_int(item, args)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn api(args: TokenStream, code: TokenStream) -> TokenStream {
    type Args = syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]>;
    let item = syn::parse_macro_input!(code as syn::ItemMod);
    let attrs = syn::parse_macro_input!(args with Args::parse_terminated);

    api::api(attrs, item).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_attribute]
pub fn model(_args: TokenStream, code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemStruct);
    model::model(item).unwrap_or_else(syn::Error::into_compile_error).into()
}

#[proc_macro_derive(Entity, attributes(entity))]
/// Derive macro generating an impl of the trait `Entity`.
///
/// You can use `#[entity(gene)]`, `#[entity(flags)]` and `#[entity(growth)]`
/// to set custom fields for these methods.
pub fn entity(code: TokenStream) -> TokenStream {
    entity::entity(code)
}

#[proc_macro_derive(Belt, attributes(belt))]
/// Derive macro generating an impl of the trait `Belt`.
/// You can use `#[belt(next)]`, `#[belt(past)]` and `#[belt(buckle)]`
/// to set custom fields for these methods.
pub fn belt(code: TokenStream) -> TokenStream {
    belt::belt(code)
}

#[proc_macro_derive(Buckle, attributes(buckle))]
pub fn buckle(code: TokenStream) -> TokenStream {
    belt::buckle(code)
}

#[proc_macro_derive(Duck, attributes(duck))]
pub fn duck(code: TokenStream) -> TokenStream {
    pond::duck(code)
}

#[proc_macro_derive(Pond, attributes(pond))]
pub fn pond(code: TokenStream) -> TokenStream {
    pond::pond(code)
}

#[proc_macro_derive(Origin, attributes(origin))]
pub fn origin(code: TokenStream) -> TokenStream {
    pond::origin(code)
}

#[proc_macro_derive(ShahSchema)]
pub fn schema(code: TokenStream) -> TokenStream {
    schema::schema(code)
}

#[proc_macro]
pub fn routes(code: TokenStream) -> TokenStream {
    routes::routes(code)
}

#[proc_macro]
pub fn perms(code: TokenStream) -> TokenStream {
    perms::perms(code)
}

fn crate_ident() -> syn::Ident {
    // let found_crate = crate_name("shah").unwrap();
    // let name = match &found_crate {
    //     FoundCrate::Itself => "shah",
    //     FoundCrate::Name(name) => name,
    // };
    ident!("shah")
}

macro_rules! ident {
    ($name:literal) => {
        syn::Ident::new($name, proc_macro2::Span::call_site())
    };
}
pub(crate) use ident;

macro_rules! err {
    ($span:expr, $($msg:literal),*) => {
        Err(syn::Error::new($span, concat!( $($msg),* )))
    };
    ($span:expr, $msg:expr) => {
        Err(syn::Error::new($span, $msg))
    };
}

pub(crate) use err;
