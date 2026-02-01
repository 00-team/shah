use crate::err;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::format_ident;
use quote_into::quote_into;
use syn::spanned::Spanned;

pub(crate) type Args =
    syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]>;

pub(crate) fn flags(
    args: Args, mut item: syn::ItemStruct,
) -> syn::Result<TokenStream2> {
    let mut s = TokenStream2::new();
    // let args = syn::parse::<syn::Meta>(args)?;
    // #[shah::flags(inner = [u8; 32], bits = 2, serde = "both")]

    if !item.attrs.is_empty() {
        return err!(item.attrs.first().span(), "remove derives");
    }

    let ci = crate::crate_ident();

    let args = parse_args(args)?;
    let vis = item.vis.clone();
    let name = item.ident.clone();
    let inner = &args.inner;

    item.ident = format_ident!("{}Info", item.ident);

    let info_name = &item.ident;

    let mut imp = TokenStream2::new();
    let mut apply = TokenStream2::new();
    let mut from_main = TokenStream2::new();
    let mut key_val = TokenStream2::new();

    let key_val_len = item.fields.len();
    let mut do_key_val = true;
    let mut key_val_ty = None;

    let mut bit_offset = 0;
    for f in item.fields.iter_mut() {
        let Some(fname) = &f.ident else { return err!(f.span(), "no name") };
        let vis = &f.vis;
        let fty = &f.ty;

        if do_key_val {
            if let Some(kvt) = &key_val_ty {
                if fty != kvt {
                    do_key_val = false;
                }
            } else {
                key_val_ty = Some(fty.clone());
            }
        }

        let setter = format_ident!("set_{fname}");
        let mut bits = args.bits;
        let mut skip_apply = false;
        for attr in f.attrs.iter() {
            if !attr.path().is_ident("flags") {
                continue;
            }
            let pfa = parse_fa(attr.parse_args_with(Args::parse_terminated)?)?;
            bits = pfa.bits.unwrap_or(bits);
            skip_apply = pfa.skip_apply;
        }
        f.attrs.retain(|a| !a.path().is_ident("flags"));

        if bit_offset + bits > args.max_bits {
            return err!(
                fname.span(),
                format!(
                    "maximum capacity of type: {} = {} bits has reached",
                    quote::quote!(#inner).to_string(),
                    args.max_bits
                )
            );
        }

        type Fq = fn(&mut TokenStream2, usize, usize);
        let (fg, fs): (Fq, Fq) = if args.is_array {
            (quote_array_get, quote_array_set)
        } else {
            (quote_get, quote_set)
        };

        quote_into! {imp +=
            #vis fn #fname(&self) -> #fty {
                let out = #{fg(imp, bit_offset, bits)};
                #{if bits > 1 {
                    quote_into!(imp += #fty::from(out as u8));
                } else {
                    quote_into!(imp += out);
                }}
            }
            #vis fn #setter(&mut self, value: #fty) -> &mut Self {
                #{if bits > 1 {
                    quote_into!(imp += let value = u8::from(value););
                    if !args.is_array {
                        quote_into!(imp += let value = value as #inner;);
                    }
                } else {
                    // quote_into!(imp += let value = bool::from(value););
                }}


                {#{fs(imp, bit_offset, bits)}};

                self
            }
        };

        if args.serde {
            if !skip_apply {
                quote_into! {apply +=
                    if let Some(v) = self.#fname {
                        item.#setter(v);
                    }
                };
            }

            quote_into! {from_main += #fname: value.#fname(),};
        }

        if do_key_val {
            let fname_str = fname.to_string();
            quote_into! {key_val += (#fname_str, self.#fname),};
        }

        bit_offset += bits;
    }

    let mut item_input = item.clone();
    for f in item_input.fields.iter_mut() {
        let ty = f.ty.clone();
        f.ty = syn::parse_quote!(Option<#ty>);
    }
    item_input.ident = format_ident!("{name}Input");
    let input_name = &item_input.ident;

    let info_name_str = info_name.to_string();
    quote_into! {s +=
        #[repr(C)]
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
        #{if args.serde {
            quote_into! {s +=
                #[derive(serde::Serialize)]
                #[serde(into = #info_name_str)]
            };
        }}
        #vis struct #name {
            inner: #inner,
        }

        impl From<#inner> for #name {
            fn from(inner: #inner) -> Self {
                Self { inner }
            }
        }

        impl #name {
            #imp

            pub fn clear(&mut self) {
                self.inner = Default::default();
            }
        }

        impl #ci::models::ShahSchema for #name {
            fn shah_schema() -> #ci::models::Schema {#{if args.is_array {
                let len = (args.max_bits / 8) as u64;
                quote_into! {s +=
                    #ci::models::Schema::Array {
                        is_str: false,
                        length: #len,
                        kind: Box::new(#ci::models::Schema::U8),
                    }
                };
            } else {
                let syn::Type::Path(p) = args.inner else { unreachable!() };
                let k = p.path.get_ident().unwrap().to_string().to_uppercase();
                let kind = format_ident!("{k}");
                quote_into! {s += #ci::models::Schema::#kind};
            }}}
        }
    };

    if args.serde {
        quote_into! {s +=
            #[derive(Debug, Default, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
            #[allow(dead_code)]
            #item

            #[derive(Debug, Default, serde::Deserialize, utoipa::ToSchema)]
            #[serde(default)]
            #[allow(dead_code)]
            #item_input

            impl #input_name {
                #vis fn apply(&self, item: &mut #name) {
                    #apply
                }

                #{if do_key_val && let Some(ty) = key_val_ty {quote_into!{s +=
                    #vis fn key_val(&self) -> [(&'static str, Option<#ty>); #key_val_len] {
                        [#key_val]
                    }

                    #vis fn key_val_some(&self) -> Vec<(&'static str, #ty)> {
                        let kv = self.key_val();
                        let mut out = Vec::with_capacity(kv.len());
                        for (k, v) in kv {
                            let Some(v) = v else {continue};
                            out.push((k, v));
                        }

                        out
                    }
                }}}

            }


            impl From<#name> for #info_name {
                fn from(value: #name) -> Self {
                    Self::from(&value)
                }
            }

            impl From<&#name> for #info_name {
                fn from(value: &#name) -> Self {
                    Self {#from_main}
                }
            }


            impl utoipa::__dev::ComposeSchema for #name {
                fn compose(
                    _: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
                ) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
                    <#info_name as utoipa::PartialSchema>::schema()
                }
            }

            impl utoipa::ToSchema for #name {
                fn schemas(
                    schemas: &mut Vec<(
                        String,
                        utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
                    )>,
                ) {
                    <#info_name as utoipa::ToSchema>::schemas(schemas)
                }
            }
        };
    }

    Ok(s)
}

struct ParsedArgs {
    inner: syn::Type,
    bits: usize,
    serde: bool,
    max_bits: usize,
    is_array: bool,
}

fn parse_args(args: Args) -> syn::Result<ParsedArgs> {
    let mut pa = ParsedArgs {
        inner: syn::parse_quote!(u32),
        bits: 1,
        serde: true,
        max_bits: 32,
        is_array: false,
    };

    for a in args {
        const KEY_ERR: &str = "key must be one of: inner,bits,serde";
        let Some(id) = a.path.get_ident() else {
            return err!(a.path.span(), KEY_ERR);
        };
        match id.to_string().as_str() {
            "inner" => {
                match a.value {
                    syn::Expr::Repeat(v) => {
                        let syn::Expr::Path(p) = *v.expr else {
                            return err!(v.span(), "invalid array type");
                        };
                        if !p.path.is_ident("u8") {
                            return err!(p.span(), "array type must be u8");
                        }
                        let syn::Expr::Lit(lit) = *v.len else {
                            return err!(
                                v.len.span(),
                                "array len must be literal"
                            );
                        };
                        let syn::Lit::Int(int) = lit.lit else {
                            return err!(
                                lit.span(),
                                "only numbers are allowed"
                            );
                        };

                        pa.is_array = true;
                        pa.max_bits = int.base10_parse::<usize>()? * 8;
                        pa.inner = syn::parse_quote!([u8; #int]);
                        // pa.inner = syn::Type::Path(syn::TypePath {
                        //     qself: None,
                        //     path: p.path,
                        // });
                    }
                    syn::Expr::Path(v) => {
                        const E: &str = "type must be a u8,u16,u32 or u64";
                        let Some(tp) = v.path.get_ident() else {
                            return err!(v.span(), E);
                        };
                        pa.max_bits = match tp.to_string().as_str() {
                            "u8" => 8,
                            "u16" => 16,
                            "u32" => 32,
                            "u64" => 64,
                            _ => return err!(tp.span(), E),
                        };
                        pa.inner = syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: v.path,
                        });
                    }
                    v => {
                        return err!(
                            v.span(),
                            "only a type of [u8; N] or u8,u16,u32,u64 is allowed"
                        );
                    }
                };
            }
            "bits" => {
                let syn::Expr::Lit(lit) = a.value else {
                    return err!(a.value.span(), "bits must be a int literal");
                };
                let syn::Lit::Int(int) = lit.lit else {
                    return err!(lit.span(), "only numbers are allowed");
                };
                pa.bits = int.base10_parse::<usize>()?;
            }
            "serde" => {
                let syn::Expr::Lit(lit) = a.value else {
                    return err!(
                        a.value.span(),
                        "serde must be a bool literal"
                    );
                };
                let syn::Lit::Bool(val) = lit.lit else {
                    return err!(lit.span(), "only bool are allowed");
                };
                pa.serde = val.value;
            }
            k => {
                return err!(
                    a.path.span(),
                    format!(
                        "unknown key of: {k}, must be one of: inner,bits,serde"
                    )
                );
            }
        }
    }

    Ok(pa)
}

struct ParsedFieldArgs {
    bits: Option<usize>,
    skip_apply: bool,
}

fn parse_fa(args: Args) -> syn::Result<ParsedFieldArgs> {
    let mut pa = ParsedFieldArgs { bits: None, skip_apply: false };

    for a in args {
        const KEY_ERR: &str = "key must be one of: bits,skip_apply";
        let Some(id) = a.path.get_ident() else {
            return err!(a.path.span(), KEY_ERR);
        };
        match id.to_string().as_str() {
            "bits" => {
                let syn::Expr::Lit(lit) = a.value else {
                    return err!(a.value.span(), "bits must be a int literal");
                };
                let syn::Lit::Int(int) = lit.lit else {
                    return err!(lit.span(), "only numbers are allowed");
                };
                pa.bits = Some(int.base10_parse::<usize>()?);
            }
            "skip_apply" => {
                let syn::Expr::Lit(lit) = a.value else {
                    return err!(a.value.span(), "value must be a bool");
                };
                let syn::Lit::Bool(val) = lit.lit else {
                    return err!(lit.span(), "only bool are allowed");
                };
                pa.skip_apply = val.value;
            }
            k => {
                return err!(
                    a.path.span(),
                    format!(
                        "unknown key of: {k}, must be one of: bits,skip_apply"
                    )
                );
            }
        }
    }

    Ok(pa)
}

fn quote_array_get(s: &mut TokenStream2, offset: usize, bits: usize) {
    if bits == 1 {
        let (byte, bit) = (offset / 8, offset % 8);
        quote_into! {s += (self.inner[#byte] & (1 << #bit)) == (1 << #bit)};
        return;
    }

    for n in 0..bits {
        let i = offset + n;
        let (byte, bit) = (i / 8, i % 8);
        quote_into! {s +=
            (((self.inner[#byte] >> #bit) & 1) << #n)
        }
        if n != bits - 1 {
            quote_into!(s += |)
        }
    }
}

fn quote_array_set(s: &mut TokenStream2, offset: usize, bits: usize) {
    if bits == 1 {
        let (byte, bit) = (offset / 8, (offset % 8) as u8);
        quote_into! {s +=
            if value {
                self.inner[#byte] |= (1 << #bit);
            } else {
                self.inner[#byte] &= !(1 << #bit);
            }
        };
        return;
    }

    for n in 0..bits {
        let i = offset + n;
        let (byte, bit) = (i / 8, (i % 8) as u8);
        quote_into! {s +=
            self.inner[#byte] = (
                (self.inner[#byte] & !(1 << #bit)) | (((value >> #n) & 1) << #bit)
            );
        }
    }
}

fn quote_get(s: &mut TokenStream2, offset: usize, bits: usize) {
    let pos = Literal::usize_unsuffixed(offset);

    if bits == 1 {
        quote_into! {s += (self.inner & (1 << #pos)) == (1 << #pos) };
        return;
    }

    let mask = Literal::u8_unsuffixed((1 << bits) - 1);
    quote_into! {s += ((self.inner >> #pos) & #mask) };
}

fn quote_set(s: &mut TokenStream2, offset: usize, bits: usize) {
    let pos = Literal::usize_unsuffixed(offset);
    if bits == 1 {
        quote_into! {s +=
            if value {
                self.inner |= (1 << #pos);
            } else {
                self.inner &= !(1 << #pos);
            }
        };
        return;
    }

    let mask = Literal::u8_unsuffixed((1 << bits) - 1);
    quote_into! {s +=
        let new = value & #mask;
        let old = self.inner & !(#mask << #pos);
        // self.inner = old | ((new as #inner) << #pos);
        self.inner = old | (new << #pos);
    };
}
