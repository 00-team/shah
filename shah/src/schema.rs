use crate::error::{ShahError, SystemError};

#[derive(Debug, crate::EnumCode, PartialEq, Eq)]
pub enum Schema {
    Model(SchemaModel),
    Array { length: u64, kind: Box<Schema> },
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32, // 10
    F64,
    Bool,
    Gene, // 13
}

impl Schema {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![u16::from(self) as u8];

        fn check_schema(out: &mut Vec<u8>, schema: &Schema) {
            match schema {
                Schema::Model(_) | Schema::Array { .. } => {
                    out.extend_from_slice(&Schema::encode(schema));
                }
                _ => {
                    out.push(u16::from(schema) as u8);
                }
            }
        }

        match self {
            Self::Model(m) => {
                out.extend_from_slice(m.name.as_bytes());
                out.push(0);
                out.extend_from_slice(&m.size.to_le_bytes());
                out.extend_from_slice(&(m.fields.len() as u16).to_le_bytes());
                for (ident, ty) in m.fields.iter() {
                    out.extend_from_slice(ident.as_bytes());
                    out.push(0);
                    check_schema(&mut out, ty);
                }
            }
            Self::Array { length, kind } => {
                out.extend_from_slice(&(*length).to_le_bytes());
                check_schema(&mut out, &kind);
            }
            _ => {}
        }
        out
    }

    fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            2 => Self::U8,
            3 => Self::U16,
            4 => Self::U32,
            5 => Self::U64,
            6 => Self::I8,
            7 => Self::I16,
            8 => Self::I32,
            9 => Self::I64,
            10 => Self::F32,
            11 => Self::F64,
            12 => Self::Bool,
            13 => Self::Gene,
            _ => return None,
        })
    }

    fn from_iter(it: &mut core::slice::Iter<u8>) -> Option<Self> {
        macro_rules! from_iter {
            (str) => {{
                let before = it.as_slice();
                let pos = it.position(|a| *a == 0)?;
                let res = String::from_utf8(before[..pos].to_vec()).ok()?;
                res
            }};
            ($ty:ty) => {{
                let mut size = [0u8; core::mem::size_of::<$ty>()];
                for s in size.iter_mut() {
                    *s = *it.next()?;
                }
                <$ty>::from_le_bytes(size)
            }};
        }

        loop {
            match *it.next()? {
                0 => {
                    let name = from_iter!(str);
                    let size = from_iter!(u64);
                    let fields_len = from_iter!(u16);
                    let mut fields = Vec::<(String, Schema)>::with_capacity(
                        fields_len as usize,
                    );
                    for _ in 0..fields_len {
                        let ident = from_iter!(str);
                        let kind = match *it.clone().next()? {
                            0 | 1 => Self::from_iter(it)?,
                            c => {
                                it.next();
                                Self::from_code(c)?
                            }
                        };
                        fields.push((ident, kind));
                    }
                    return Some(Schema::Model(SchemaModel {
                        name,
                        size,
                        fields,
                    }));
                }
                1 => {
                    let length = from_iter!(u64);
                    let kind = Box::new(match *it.clone().next()? {
                        0 | 1 => Self::from_iter(it)?,
                        c => {
                            it.next();
                            Self::from_code(c)?
                        }
                    });
                    return Some(Schema::Array { length, kind });
                }
                _ => break,
            }
        }

        None
    }

    pub fn decode(value: &[u8]) -> Result<Self, ShahError> {
        let Some(schema) = Self::from_iter(&mut value.iter()) else {
            return Err(SystemError::InvalidSchemaData)?;
        };
        Ok(schema)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SchemaModel {
    pub name: String,
    pub size: u64,
    pub fields: Vec<(String, Schema)>,
}

pub trait ShahSchema {
    fn shah_schema() -> Schema;
}

macro_rules! impl_primitive {
    ($($ty:ty, $variant:ident,)*) => {
        $(
        impl ShahSchema for $ty {
            fn shah_schema() -> Schema {
                Schema::$variant
            }
        }
        )*
    };
}

impl_primitive! {
    u8, U8,
    u16, U16,
    u32, U32,
    u64, U64,
    i8,  I8,
    i16, I16,
    i32, I32,
    i64, I64,
    f32, F32,
    f64, F64,
    bool, Bool,
}
