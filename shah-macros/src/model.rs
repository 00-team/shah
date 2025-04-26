use crate::{err, utils::args::args_parse};
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use quote_into::quote_into;
use syn::{parse::Parser, spanned::Spanned};

pub(crate) fn model(mut item: syn::ItemStruct) -> syn::Result<TokenStream2> {
    let (impl_gnc, ty_gnc, where_gnc) = item.generics.split_for_impl();
    let is_generic = item.generics.lt_token.is_some();

    if !matches!(item.fields, syn::Fields::Named(_)) {
        return err!(item.span(), "invalid struct type must be named");
    }

    for attr in item.attrs.iter() {
        let syn::Meta::List(meta) = &attr.meta else {
            continue;
        };

        let ident = meta.path.segments[0].ident.to_string();
        if ident == "repr" {
            return err!(attr.span(), "model must be repr(C) which is default");
        }
        if ident == "derive" {
            for token in meta.tokens.clone() {
                let proc_macro2::TokenTree::Ident(t) = &token else {
                    continue;
                };

                match t.to_string().as_str() {
                    "Default" | "Copy" | "Clone" => {
                        return err!(
                            token.span(),
                            format!("remove the {t} derive")
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    item.attrs.push(syn::parse_quote! { #[derive(Copy)] });
    item.attrs.push(syn::parse_quote! { #[repr(C)] });

    let ident = item.ident.clone();
    let ci = crate::crate_ident();

    let mut flags_fields = Vec::<FlagsField>::with_capacity(5);
    let mut str_fields = Vec::<StrField>::with_capacity(5);

    for f in item.fields.iter_mut() {
        let mut retained_attrs = Vec::with_capacity(f.attrs.len());
        for attr in &f.attrs {
            let Some(ma) = parse_attrs(f, attr)? else {
                retained_attrs.push(attr.clone());
                continue;
            };
            match ma {
                ModelAttr::Str(v) => str_fields.push(v),
                ModelAttr::Flags(v) => flags_fields.push(v),
            }
        }

        f.attrs = retained_attrs;
    }

    // let fields_len = item.fields.len();

    let mut assprv: Option<(&syn::Ident, &syn::Type)> = None;
    let mut asspad = TokenStream2::new();
    let assid = if is_generic { crate::ident!("Self") } else { ident.clone() };
    for f in item.fields.iter() {
        let Some(fid) = &f.ident else {
            return err!(f.span(), "field must have an ident");
        };
        let Some((pid, pty)) = assprv else {
            assprv = Some((fid, &f.ty));
            quote_into! { asspad +=
                assert!(::core::mem::offset_of!(#assid, #fid) == 0);
            }
            continue;
        };

        assprv = Some((fid, &f.ty));
        quote_into! { asspad +=
            assert!(::core::mem::offset_of!(#assid, #fid) ==
                ::core::mem::offset_of!(#assid, #pid) +
                ::core::mem::size_of::<#pty>()
            );
        }
    }

    if let Some((lid, lty)) = assprv {
        quote_into! { asspad +=
            assert!(
                ::core::mem::size_of::<#assid>() ==
                ::core::mem::offset_of!(#assid, #lid) +
                ::core::mem::size_of::<#lty>()
            );
        }
    }

    let mut default_impl = TokenStream2::new();
    for f in item.fields.iter() {
        let fi = &f.ident;
        match &f.ty {
            syn::Type::Path(_) => {
                quote_into!(default_impl += #fi: ::core::default::Default::default(),)
            }
            syn::Type::Array(a) => {
                let len = &a.len;
                let el = &a.elem;
                // let at = &path(&a.elem).path.segments[0].ident;
                quote_into!(default_impl += #fi: [<#el>::default(); #len],)
            }
            syn::Type::Tuple(t) => {
                quote_into! {default_impl += #fi: (#{
                    t.elems.iter().for_each(|e| quote_into!{default_impl += <#e>::default(),})
                }),}
            }
            t => {
                panic!("unknown type for default impl: {}", t.to_token_stream())
            }
        }
    }

    let mut ssi = TokenStream2::new();
    for StrField { field, get, set } in str_fields.iter() {
        if *get {
            quote_into! {ssi +=
                pub fn #field<'a>(&'a self) -> &'a str {
                    #ci::AsUtf8Str::as_utf8_str_null_terminated(&self.#field)
                }
            };
        }
        if *set {
            let set_ident = format_ident!("set_{field}");
            quote_into! {ssi +=
                pub fn #set_ident(&mut self, value: &str) -> bool {
                    if value.is_empty() {
                        self.#field.fill(0);
                        return false;
                    }

                    let mut overflow = false;
                    let vlen = value.len();
                    let flen = self.#field.len();
                    let len = if vlen > flen {
                        overflow = true;
                        let mut idx = flen;
                        loop {
                            if value.is_char_boundary(idx) {
                                break idx;
                            }
                            idx -= 1;
                            continue;
                        }
                    } else {
                        vlen
                    };

                    self.#field[..len].copy_from_slice(&value.as_bytes()[..len]);
                    if len < flen {
                        self.#field[len] = 0;
                    }

                    overflow
                }
            };
        }
    }

    let mut ffs = TokenStream2::new();
    for ff in &flags_fields {
        ff.quote_into(&mut ffs);
    }

    let mut s = quote! {
        #item

        #[automatically_derived]
        impl #impl_gnc ::core::default::Default for #ident #ty_gnc #where_gnc {
            #[inline]
            fn default() -> Self {
                #ident {#default_impl}
            }
        }

        #[automatically_derived]
        impl #impl_gnc #ident #ty_gnc #where_gnc {
            #ssi
            #ffs
        }

        #[automatically_derived]
        impl #impl_gnc Clone for #ident #ty_gnc #where_gnc {
            fn clone(&self) -> Self {
                *self
            }
        }

        #[automatically_derived]
        impl #impl_gnc #ci::models::Binary for #ident #ty_gnc #where_gnc {}

        #[automatically_derived]
        impl #impl_gnc #ci::ShahModel for #ident #ty_gnc #where_gnc {}
    };

    if is_generic {
        quote_into! {s +=
            impl #impl_gnc #ident #ty_gnc #where_gnc {
                pub const fn __assert_padding() {
                    #asspad
                }
            }
        };
    } else {
        quote_into! {s +=
            const _: () = { #asspad };
        };
    }

    Ok(s)
}

struct StrField {
    field: syn::Ident,
    get: bool,
    set: bool,
}

struct FlagsField {
    field: syn::Ident,
    flags: Vec<(syn::Ident, syn::Ident)>,
    flag_ty: Option<syn::Ident>,
    bits: u8,
}

impl FlagsField {
    fn quote_array(&self, s: &mut TokenStream2) {
        let field = &self.field;
        for (i, (get, set)) in self.flags.iter().enumerate() {
            let (byte, bit) = (i / 8, i % 8);
            quote_into! {s +=
                pub fn #get(&self) -> bool {
                    (self.#field[#byte] & (1 << #bit)) == (1 << #bit)
                }

                pub fn #set(&mut self, #get: bool) -> &mut Self {
                    if #get {
                        self.#field[#byte] |= (1 << #bit);
                    } else {
                        self.#field[#byte] &= !(1 << #bit);
                    }
                    self
                }
            };
        }
    }

    fn quote_array_bits(&self, s: &mut TokenStream2) {
        let field = &self.field;
        // let mask = Literal::u8_unsuffixed((1 << self.bits) - 1);
        let ubits = self.bits as usize;

        for (x, (get, set)) in self.flags.iter().enumerate() {
            // let pos = Literal::usize_unsuffixed(i * self.bits as usize);
            quote_into! {s +=
                pub fn #get(&self) -> u8 {
                    #{for n in 0..ubits {
                        let i = x * ubits + n;
                        let (byte, bit) = (i / 8, i % 8);
                        // let rn = ubits - n - 1;
                        quote_into! {s +=
                            (((self.#field[#byte] >> #bit) & 1) << #n)
                        }
                        if n != ubits - 1 {
                            quote_into!(s += |)
                        }
                    }}
                }

                pub fn #set(&mut self, #get: u8) -> &mut Self {
                    #{for n in 0..ubits {
                        let i = x * ubits + n;
                        let (byte, bit) = (i / 8, i % 8);
                        quote_into! {s +=
                            self.#field[#byte] = (
                                (self.#field[#byte] & !(1 << #bit)) |
                                (((#get >> #n) & 1) << #bit)
                            );
                        }
                    }}

                    self
                }
            };
        }
    }

    fn quote_bits(&self, s: &mut TokenStream2, ty: &syn::Ident) {
        let field = &self.field;
        let mask = Literal::u8_unsuffixed((1 << self.bits) - 1);

        for (i, (get, set)) in self.flags.iter().enumerate() {
            let pos = Literal::usize_unsuffixed(i * self.bits as usize);
            quote_into! {s +=
                pub fn #get(&self) -> u8 {
                    ((self.#field >> #pos) & #mask) as u8
                }

                pub fn #set(&mut self, #get: u8) -> &mut Self {
                    let new = #get & #mask;
                    let old = self.#field & !(#mask << #pos);
                    self.#field = old | ((new as #ty) << #pos);

                    self
                }
            };
        }
    }

    pub fn quote_into(&self, s: &mut TokenStream2) {
        let Some(ty) = &self.flag_ty else {
            if self.bits == 1 {
                return self.quote_array(s);
            }
            return self.quote_array_bits(s);
        };

        if self.bits != 1 {
            return self.quote_bits(s, ty);
        }

        let field = &self.field;
        for (i, (get, set)) in self.flags.iter().enumerate() {
            quote_into! {s +=
                pub fn #get(&self) -> bool {
                    (self.#field & (1 << #i)) == (1 << #i)
                }

                pub fn #set(&mut self, #get: bool) -> &mut Self {
                    if #get {
                        self.#field |= (1 << #i);
                    } else {
                        self.#field &= !(1 << #i);
                    }
                    self
                }
            };
        }
    }
}

enum ModelAttr {
    Str(StrField),
    Flags(FlagsField),
}

args_parse! {
    #[derive(Debug)]
    struct StrArgs {
        get: Option<syn::LitBool>,
        set: Option<syn::LitBool>,
    }
}

fn parse_str_attr(
    f: &syn::Field, attr: &syn::Attribute,
) -> syn::Result<StrField> {
    const TYERR: &str = "#[str] field type must be an array of [u8; ..]";
    match &f.ty {
        syn::Type::Array(a) => match &(*a.elem) {
            syn::Type::Path(p) => {
                if !p.path.is_ident("u8") {
                    return err!(a.span(), TYERR);
                }
            }
            _ => return err!(a.span(), TYERR),
        },
        _ => return err!(f.ty.span(), TYERR),
    }

    match &attr.meta {
        syn::Meta::Path(_) => Ok(StrField {
            field: f.ident.clone().unwrap(),
            get: true,
            set: true,
        }),
        syn::Meta::List(ml) => {
            let args: StrArgs = syn::parse(ml.tokens.clone().into())?;
            Ok(StrField {
                field: f.ident.clone().unwrap(),
                get: args.get.map(|v| v.value).unwrap_or(true),
                set: args.set.map(|v| v.value).unwrap_or(true),
            })
        }
        _ => err!(attr.span(), "invalid attribute for #[str]"),
    }
}

type FlagsList = syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>;

fn parse_flags_attr(
    f: &syn::Field, attr: &syn::Attribute,
) -> syn::Result<FlagsField> {
    const ARRERR: &str = "#[flags] array value must be [u8; ...]";
    const ARGERR: &str = "#[flags(f1, fl2, bits = 2, f3)] is only valid";
    let flag_ty = match &f.ty {
        syn::Type::Array(a) => {
            let syn::Type::Path(p) = &(*a.elem) else {
                return err!(a.span(), ARRERR);
            };
            if !p.path.is_ident("u8") {
                return err!(a.span(), ARRERR);
            }
            None
        }
        syn::Type::Path(p) => p.path.get_ident().cloned(),
        _ => return err!(f.ty.span(), "invalid type for #[flags]"),
    };

    match &attr.meta {
        syn::Meta::List(ml) => {
            let parser = FlagsList::parse_terminated;
            let keys = parser.parse(ml.tokens.clone().into());
            let Ok(keys) = keys else {
                return err!(attr.span(), ARGERR);
            };

            let mut ff = FlagsField {
                field: f.ident.clone().unwrap(),
                flags: Vec::with_capacity(keys.len()),
                flag_ty,
                bits: 1,
            };

            for exp in &keys {
                match exp {
                    syn::Expr::Path(ep) => {
                        let i = ep.path.get_ident().unwrap();
                        ff.flags.push((i.clone(), format_ident!("set_{i}")));
                    }
                    syn::Expr::Assign(ea) => {
                        let syn::Expr::Path(left) = &(*ea.left) else {
                            return err!(ml.span(), ARGERR);
                        };
                        if !left.path.is_ident("bits") {
                            return err!(ml.span(), ARGERR);
                        }
                        let syn::Expr::Lit(right) = &(*ea.right) else {
                            return err!(
                                ml.span(),
                                "value of bits must be literal"
                            );
                        };
                        let syn::Lit::Int(bits) = &right.lit else {
                            return err!(
                                ml.span(),
                                "value of bits must be ",
                                "a number in range of 2..8"
                            );
                        };
                        ff.bits = bits.base10_parse::<u8>()?;
                        if ff.bits > 7 || ff.bits < 2 {
                            return err!(
                                ml.span(),
                                "value of bits must be in range of 2..8"
                            );
                        }
                    }
                    _ => return err!(ml.span(), ARGERR),
                }
            }

            Ok(ff)
        }
        _ => {
            err!(attr.span(), "#[flags] attr must have at least have one flag")
        }
    }
}

fn parse_attrs(
    f: &syn::Field, attr: &syn::Attribute,
) -> syn::Result<Option<ModelAttr>> {
    let attr_path = attr.path();
    if attr_path.is_ident("str") {
        return Ok(Some(ModelAttr::Str(parse_str_attr(f, attr)?)));
    }
    if attr_path.is_ident("flags") {
        return Ok(Some(ModelAttr::Flags(parse_flags_attr(f, attr)?)));
    }

    Ok(None)
}
