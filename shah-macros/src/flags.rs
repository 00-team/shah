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

    let args = parse_args(args)?;
    let vis = item.vis.clone();
    let name = item.ident.clone();
    let inner = &args.inner;
    item.ident = format_ident!("{}Info", item.ident);
    let info_name = &item.ident;

    let mut imp = TokenStream2::new();
    let mut apply = TokenStream2::new();
    let mut from_main = TokenStream2::new();

    let mut bit_offset = 0;
    for f in item.fields.iter() {
        let Some(fname) = &f.ident else { return err!(f.span(), "no name") };
        let vis = &f.vis;
        let fty = &f.ty;
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

        type Fq = fn(&mut TokenStream2, usize, usize);
        let (fg, fs): (Fq, Fq) = if args.is_array {
            (quote_array_get, quote_array_set)
        } else {
            (quote_get, quote_set)
        };

        quote_into! {imp +=
            #vis fn #fname(&self) -> #fty {
                #fty::from((#{fg(imp, bit_offset, bits)}))
            }
            #vis fn #setter(&mut self, value: #fty) -> &mut Self {
                #{if bits > 1 {
                    quote_into!(imp += let value = u8::from(value););
                } else {
                    quote_into!(imp += let value = bool::from(value););
                }}

                {#{fs(imp, bit_offset, bits)}};

                self
            }
        };

        if args.serde {
            if !skip_apply {
                quote_into! {apply += item.#setter(self.#fname);};
            }

            quote_into! {from_main += #fname: item.#fname(),};
        }

        bit_offset += bits;
    }

    let info_name_str = info_name.to_string();
    quote_into! {s +=
        #[repr(C)]
        #[derive(Debug, Default, Clone, Copy)]
        #{if args.serde {
            quote_into! {s +=
                #[derive(serde::Serialize)]
                #[serde(into = #info_name_str)]
            };
        }}
        #vis struct #name {
            inner: #inner,
        }

        impl #name {
            #imp
        }
    };

    if args.serde {
        quote_into! {s +=
            #item

            impl #info_name {
                #vis fn apply(&self, item: &mut #name) {
                    #apply
                }
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
                        pa.inner = syn::Type::Path(syn::TypePath {
                            qself: None,
                            path: p.path,
                        });
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
        let (byte, bit) = (offset / 8, offset % 8);
        quote_into! {s +=
            if bool::from(value) {
                self.inner[#byte] |= (1 << #bit);
            } else {
                self.inner[#byte] &= !(1 << #bit);
            }
        };
        return;
    }

    for n in 0..bits {
        let i = offset + n;
        let (byte, bit) = (i / 8, i % 8);
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
    if bits == 1 {
        quote_into! {s +=
            if bool::form(value) {
                self.inner |= (1 << #offset);
            } else {
                self.inner &= !(1 << #offset);
            }
        };
        return;
    }

    let mask = Literal::u8_unsuffixed((1 << bits) - 1);
    let pos = Literal::usize_unsuffixed(offset);
    quote_into! {s +=
        let new = value & #mask;
        let old = self.inner & !(#mask << #pos);
        // self.inner = old | ((new as #inner) << #pos);
        self.inner = old | (new << #pos);
    };
}
