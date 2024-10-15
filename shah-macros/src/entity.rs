use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote_into::quote_into;

pub(crate) fn entity(_args: TokenStream, code: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(code as syn::ItemStruct);
    let mut s = TokenStream2::new();
    let ident = &item.ident;

    quote_into! {s +=
        #item

        impl #ci::entity::Entity for #ident {
            fn gene(&self) -> &Gene {
                &self.gene
            }
            fn flags(&self) -> &u8 {
                &self.flags.as_binary()[0]
            }

            fn gene_mut(&mut self) -> &mut Gene {
                &mut self.gene
            }
            fn flags_mut(&mut self) -> &mut u8 {
                &mut self.flags.as_binary_mut()[0]
            }
        }
    };

    s.into()
}

#[derive(Debug)]
enum IntKind {
    Unsigned,
    Signed,
    Float,
}

#[derive(Debug)]
enum OrganKind {
    Int { kind: IntKind, bits: u16 },
    Str { len: u64 },
    // Byte { len: u64 },
    Struct { ident: Ident },
}

impl TryFrom<syn::TypePath> for OrganKind {
    type Error = &'static str;

    fn try_from(ty: syn::TypePath) -> Result<Self, Self::Error> {
        let ty_ident = ty.path.segments[0].ident.clone();
        let ty = ty_ident.to_string();
        let mut tyc = ty.chars();
        let first_char = tyc.next().unwrap();
        if first_char.is_uppercase() {
            return Ok(Self::Struct { ident: ty_ident });
        }

        match first_char {
            'u' => Ok(Self::Int {
                bits: String::from_iter(tyc)
                    .parse::<u16>()
                    .expect("invalid type"),
                kind: IntKind::Unsigned,
            }),
            'i' => Ok(Self::Int {
                bits: String::from_iter(tyc)
                    .parse::<u16>()
                    .expect("invalid type"),
                kind: IntKind::Signed,
            }),
            'f' => Ok(Self::Int {
                bits: String::from_iter(tyc)
                    .parse::<u16>()
                    .expect("invalid type"),
                kind: IntKind::Float,
            }),
            _ => Err("invalid path type"),
        }
    }
}

struct ArrLen(u64);

impl TryFrom<syn::Expr> for ArrLen {
    type Error = &'static str;

    fn try_from(value: syn::Expr) -> Result<Self, Self::Error> {
        if let syn::Expr::Lit(lit) = value {
            if let syn::Lit::Int(int) = lit.lit {
                let len =
                    int.base10_parse::<u64>().expect("invalid array length");
                return Ok(Self(len));
            }
        }

        Err("invalid ArrLen")
    }
}

#[derive(Debug)]
struct Organ {
    ident: Ident,
    array: Vec<u64>,
    kind: OrganKind,
}

impl TryFrom<syn::Field> for Organ {
    type Error = &'static str;

    fn try_from(field: syn::Field) -> Result<Self, Self::Error> {
        let mut organ = Self {
            ident: field.ident.clone().unwrap(),
            array: vec![],
            kind: OrganKind::Str { len: 0 },
        };

        if let syn::Type::Path(p) = field.ty {
            organ.kind = OrganKind::try_from(p)?;
            return Ok(organ);
        }

        fn recursive_array(
            ty: syn::TypeArray, organ: &mut Organ,
        ) -> Result<(), &'static str> {
            let len = ArrLen::try_from(ty.len)?.0;

            if let syn::Type::Path(p) = *ty.elem {
                if p.path.segments[0].ident == "char" {
                    organ.kind = OrganKind::Str { len };
                    return Ok(());
                }

                organ.kind = OrganKind::try_from(p)?;
                organ.array.push(len);
                return Ok(());
            }

            if let syn::Type::Array(a) = *ty.elem {
                organ.array.push(len);
                return recursive_array(a, organ);
            }

            Err("invalid array field")
        }

        if let syn::Type::Array(a) = field.ty {
            recursive_array(a, &mut organ)?;
            return Ok(organ);
        }

        Err("invalid or unknown field type")
    }
}

#[derive(Debug)]
struct Organism {
    organs: Vec<Organ>,
}

impl TryFrom<syn::ItemStruct> for Organism {
    type Error = &'static str;

    fn try_from(item: syn::ItemStruct) -> Result<Self, Self::Error> {
        if !matches!(item.fields, syn::Fields::Named(_)) {
            return Err("invalid struct fields must be named");
        }

        let mut organism =
            Organism { organs: Vec::with_capacity(item.fields.len()) };

        for field in item.fields {
            organism.organs.push(Organ::try_from(field)?);
        }

        Ok(organism)
    }
}
