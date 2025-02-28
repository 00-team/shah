use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, ToTokens};
use quote_into::quote_into;

pub(crate) fn command(code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemEnum);
    let ci = crate::crate_ident();

    let mut s = TokenStream2::new();
    let ident = &item.ident;

    let mut b = TokenStream2::new();
    for v in item.variants.iter() {
        let ident = &v.ident;
        let cmd = ident_to_command(ident);
        match &v.fields {
            syn::Fields::Unit => quote_into!(b += #cmd => Self::#ident,),
            syn::Fields::Unnamed(unf) => {
                quote_into! {b += #cmd => {#{
                    let mut args = TokenStream2::new();
                    for (i, f) in unf.unnamed.iter().enumerate() {
                        let iv = format_ident!("iv{i}");
                        quote_into!(b += let Some(#iv) = args.next() else { return Self::default() };);
                        if is_number(&f.ty) {
                            let ty = &f.ty;
                            quote_into!(b += let Ok(#iv) = #iv.parse::<#ty>() else { return Self::default() };)
                        }
                        quote_into!(args += #iv,);
                    }

                    quote_into!(b += Self::#ident(#args))
                }}}
            }
            syn::Fields::Named(nf) => {
                quote_into! {b += #cmd => {#{
                    let mut args = TokenStream2::new();

                    for syn::Field { ident, ty, .. } in nf.named.iter() {
                        quote_into!(b += let mut #ident: Option<#ty> = None;)
                    }

                    quote_into! {b += loop {
                        let Some(key) = args.next() else { break };
                        let Some(val) = args.next() else { return Self::default() };

                        match key.as_str() {#{
                            for syn::Field { ident, ty, .. } in nf.named.iter() {
                                let ki = format!("--{}", ident.clone().unwrap());
                                if is_number(ty) {
                                    quote_into!(b += #ki => {
                                        let Ok(val) = val.parse::<#ty>() else { return Self::default() };
                                        #ident = Some(val);
                                    });
                                } else {
                                    quote_into!(b += #ki => { #ident = Some(val); });
                                }
                            }
                        }
                            _ => return Self::default(),
                        }
                    }};

                    for syn::Field { ident, .. } in nf.named.iter() {
                        quote_into!(b += if #ident.is_none() { return Self::default() });
                        quote_into!(args += #ident: #ident.unwrap(), );
                    }

                    quote_into!(b += Self::#ident { #args })
                }}}
            }
        }
    }

    quote_into!(b += _ => Self::default(),);

    let mut h = TokenStream2::new();
    quote_into! {h +=
        let mut help = "/path/to/bin -c <command>\nlist of commands:\n".to_string();
    };
    for v in item.variants.iter() {
        let ident = &v.ident;
        let cmd = ident_to_command(ident);
        quote_into! {h += help += "    "; help += #cmd; };
        match &v.fields {
            syn::Fields::Unnamed(unf) => {
                for syn::Field { ty, .. } in unf.unnamed.iter() {
                    let ty = ty.to_token_stream().to_string();
                    quote_into! {h += help += " <"; help += #ty; help += ">";}
                }
            }
            syn::Fields::Named(nf) => {
                for syn::Field { ident, ty, .. } in nf.named.iter() {
                    let iv = ident.clone().unwrap().to_string();
                    let ty = ty.to_token_stream().to_string();
                    quote_into! {h += help += " --"; help += #iv; help += " <"; help += #ty; help += ">";}
                }
            }
            _ => {}
        }
        quote_into!(h += help.push('\n'););
    }
    quote_into!(h += help.push('\n'); help);

    quote_into! {s +=
        #[automatically_derived]
        impl #ci::Command for #ident {
            fn parse(mut args: std::env::Args) -> Self {
                let Some(cmd) = args.next() else { return Self::default() };

                match cmd.as_str() {
                    #b
                }
            }

            fn help() -> String {
                #h
            }
        }
    };

    s.into()
}

fn ident_to_command(ident: &syn::Ident) -> String {
    let mut cmd = String::new();
    for (i, c) in ident.to_string().chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                cmd.push('-');
            }
            cmd.push(c.to_ascii_lowercase());
        } else {
            cmd.push(c);
        }
    }

    cmd
}

fn is_number(ty: &syn::Type) -> bool {
    if let syn::Type::Path(p) = ty {
        return p.path.to_token_stream().to_string() != "String";
    }

    panic!("type: {} is not supported in Command", ty.to_token_stream())
}
