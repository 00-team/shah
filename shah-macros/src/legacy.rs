use crate::err;
use proc_macro2::TokenStream as TokenStream2;
use quote_into::quote_into;
use std::collections::HashMap;
use syn::spanned::Spanned;

pub(crate) fn legacy(item: syn::ItemMod) -> syn::Result<TokenStream2> {
    let item_span = item.span();
    let Some((_, content)) = item.content else {
        return err!(item_span, "mod is empty");
    };

    if content.is_empty() {
        return err!(item_span, "mod is empty");
    }

    let mut base: Option<StructData> = None;
    let mut base_impl_from: HashMap<syn::Path, ImplFrom> = HashMap::new();
    let mut kozo: Vec<StructData> = Vec::with_capacity(10);
    let mut kozo_impl_from: HashMap<syn::Ident, Vec<ImplFrom>> = HashMap::new();

    let mut s = TokenStream2::new();

    for item in content.iter() {
        match item {
            syn::Item::Struct(s) => {
                let sd = StructData::from_item_struct(s)?;

                if sd.ident == "Base" {
                    base = Some(sd);
                } else {
                    kozo_impl_from.insert(sd.ident.clone(), Vec::new());
                    kozo.push(sd);
                }
            }
            syn::Item::Impl(i) => {
                let impf = ImplFrom::from_item_impl(i)?;
                if impf.ident == "Base" {
                    base_impl_from.insert(impf.from.clone(), impf);
                } else if let Some(x) = kozo_impl_from.get_mut(&impf.ident) {
                    x.push(impf);
                } else {
                    kozo_impl_from.insert(impf.ident.clone(), vec![impf]);
                }
            }
            syn::Item::Use(u) => {
                quote_into! {s += #u};
            }
            _ => return err!(item.span(), "invalid item"),
        }
    }

    let Some(base) = base else {
        return err!(item_span, "no struct Base was found");
    };

    for k in kozo.iter_mut() {
        let mut f = TokenStream2::new();
        base.fields.iter().for_each(|ff| quote_into!(f += #ff,));
        k.fields.iter().for_each(|ff| quote_into!(f += #ff,));

        let mut a = TokenStream2::new();
        base.attrs.iter().for_each(|aa| quote_into!(a += #aa));
        k.attrs.iter().for_each(|aa| quote_into!(a += #aa));

        let ki = &k.ident;
        quote_into!(s += #a pub struct #ki {#f});
    }

    for (_, kifs) in kozo_impl_from.iter() {
        for ImplFrom { ident, from, statements, fields } in kifs.iter() {
            let mut sm = TokenStream2::new();
            let mut ff = TokenStream2::new();
            if let Some(bif) = base_impl_from.get(from) {
                bif.statements.iter().for_each(|stmt| quote_into!(sm += #stmt));
                bif.fields.iter().for_each(|f| quote_into!(ff += #f,));
            }

            statements.iter().for_each(|stmt| quote_into!(sm += #stmt));
            fields.iter().for_each(|f| quote_into!(ff += #f,));

            quote_into! {s +=
                impl From<#from> for #ident {
                    fn from(value: #from) -> Self {
                        Self::from(&value)
                    }
                }

                impl From<&#from> for #ident {
                    fn from(value: &#from) -> Self {
                        #sm
                        Self { #ff }
                    }
                }
            }
        }
    }

    Ok(s)
}

#[derive(Debug)]
struct StructData {
    attrs: Vec<syn::Attribute>,
    ident: syn::Ident,
    fields: Vec<syn::Field>,
}

impl StructData {
    fn from_item_struct(item: &syn::ItemStruct) -> syn::Result<Self> {
        let syn::Fields::Named(n) = &item.fields else {
            return err!(item.fields.span(), "invalid struct type");
        };

        if item.generics.lt_token.is_some() {
            return err!(item.generics.span(), "generics arent supported");
        }

        Ok(Self {
            attrs: item.attrs.clone(),
            ident: item.ident.clone(),
            fields: n.named.iter().cloned().collect(),
        })
    }
}

#[derive(Debug)]
struct ImplFrom {
    ident: syn::Ident,
    from: syn::Path,
    statements: Vec<syn::Stmt>,
    fields: Vec<syn::FieldValue>,
}

impl ImplFrom {
    fn from_item_impl(item: &syn::ItemImpl) -> syn::Result<Self> {
        let Some((_, f, _)) = item.trait_.as_ref() else {
            return err!(item.span(), "impl Struct is not supported");
        };

        let fi = &f.segments[0];
        if fi.ident != "From" {
            return err!(fi.span(), "only impl From is supported");
        }
        let syn::PathArguments::AngleBracketed(ab) = &fi.arguments else {
            return err!(fi.arguments.span(), "invalid impl From");
        };

        let syn::GenericArgument::Type(aba) = &ab.args[0] else {
            return err!(ab.args.span(), "invalid impl From generics");
        };

        let syn::Type::Reference(tr) = aba else {
            return err!(aba.span(), "From type must be a reference");
        };

        let syn::Type::Path(from) = &(*tr.elem) else {
            return err!(tr.elem.span(), "From type must be a &Struct");
        };

        let syn::Type::Path(sfy) = &(*item.self_ty) else {
            return err!(tr.elem.span(), "invalid impl");
        };

        let mut impf = Self {
            ident: sfy.path.segments[0].ident.clone(),
            from: from.path.clone(),
            statements: Vec::new(),
            fields: Vec::new(),
        };

        if item.items.len() != 1 {
            return err!(tr.elem.span(), "invalid impl From");
        }

        let syn::ImplItem::Fn(ifn) = &item.items[0] else {
            return err!(tr.elem.span(), "invalid impl From");
        };

        for stmt in ifn.block.stmts.iter() {
            let syn::Stmt::Expr(expr, semi) = stmt else {
                impf.statements.push(stmt.clone());
                continue;
            };

            if semi.is_some() {
                return err!(expr.span(), "return type must be Self {..}");
            }

            let syn::Expr::Struct(es) = expr else {
                return err!(expr.span(), "return type must be Self {..}");
            };

            impf.fields = es.fields.iter().cloned().collect();
        }

        Ok(impf)
    }
}
