use crate::error::{DbError, ShahError};

use super::{Binary, Gene};

#[derive(Debug, crate::EnumCode)]
#[enum_code(u8)]
pub enum Schema {
    Model(SchemaModel),
    Array { length: u64, kind: Box<Schema> },
    Tuple(Vec<Schema>),
    String(u64),
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64, // 10
    F32,
    F64,
    Bool,
    Gene, // 14
}

impl PartialEq for Schema {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Model(sm) => matches!(other, Self::Model(om) if sm == om),
            // Self::Model(sm) => match other {
            //     Self::Model(om) => sm == om,
            //     _ => false,
            // },
            Self::Array { length: sl, kind: sk } => match other {
                Self::Array { length: ol, kind: ok } => sl == ol && sk == ok,
                _ => false,
            },
            // Self::Tuple(st) => match other {
            //     Self::Tuple(ot) => st == ot,
            //     _ => false,
            // },
            // Self::String(sl) => match other {
            //     Self::String(ol) => sl == ol,
            //     _ => false,
            // },
            Self::Tuple(st) => matches!(other, Self::Tuple(ot) if st == ot),
            Self::String(sl) => matches!(other, Self::String(ol) if sl == ol),
            Self::U8 => matches!(other, Self::U8),
            Self::U16 => matches!(other, Self::U16),
            Self::U32 => matches!(other, Self::U32),
            Self::U64 => matches!(other, Self::U64),
            Self::I8 => matches!(other, Self::I8),
            Self::I16 => matches!(other, Self::I16),
            Self::I32 => matches!(other, Self::I32),
            Self::I64 => matches!(other, Self::I64),
            Self::F32 => matches!(other, Self::F32),
            Self::F64 => matches!(other, Self::F64),
            Self::Bool => matches!(other, Self::Bool),
            Self::Gene => matches!(other, Self::Gene),
        }
    }
}

impl Schema {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![u16::from(self) as u8];

        fn check_schema(out: &mut Vec<u8>, schema: &Schema) {
            match schema {
                Schema::Model(_) | Schema::Array { .. } | Schema::Tuple(_) => {
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
                check_schema(&mut out, kind);
            }
            Self::Tuple(items) => {
                out.extend_from_slice(&(items.len() as u16).to_le_bytes());
                for ty in items.iter() {
                    check_schema(&mut out, ty);
                }
            }
            _ => {}
        }
        out
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Array { length, kind } => *length as usize * kind.size(),
            Self::String(len) => *len as usize,
            Self::U8 => 1,
            Self::I8 => 1,
            Self::Bool => 1,
            Self::U16 => 2,
            Self::I16 => 2,
            Self::U32 => 4,
            Self::I32 => 4,
            Self::F32 => 4,
            Self::U64 => 8,
            Self::I64 => 8,
            Self::F64 => 8,
            Self::Gene => Gene::S,
            Self::Tuple(v) => {
                v.iter().fold(0usize, |total, s| total + s.size())
            }
            Self::Model(m) => m.size as usize,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            code if code == Self::U8
            4 => Self::U8,
            5 => Self::U16,
            6 => Self::U32,
            7 => Self::U64,
            8 => Self::I8,
            9 => Self::I16,
            10 => Self::I32,
            11 => Self::I64,
            12 => Self::F32,
            13 => Self::F64,
            14 => Self::Bool,
            15 => Self::Gene,
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

        match *it.next()? {
            0 => {
                let name = from_iter!(str);
                let size = from_iter!(u64);
                let flen = from_iter!(u16) as usize;
                let mut fields = Vec::<(String, Schema)>::with_capacity(flen);
                for _ in 0..flen {
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

                Some(Schema::Model(SchemaModel { name, size, fields }))
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

                Some(Schema::Array { length, kind })
            }
            2 => {
                let ilen = from_iter!(u16) as usize;
                let mut items = Vec::<Schema>::with_capacity(ilen);
                for _ in 0..ilen {
                    let kind = match *it.clone().next()? {
                        0..=2 => Self::from_iter(it)?,
                        c => {
                            it.next();
                            Self::from_code(c)?
                        }
                    };
                    items.push(kind);
                }

                Some(Schema::Tuple(items))
            }
            _ => None,
        }
    }

    pub fn decode(value: &[u8]) -> Result<Self, ShahError> {
        let Some(schema) = Self::from_iter(&mut value.iter()) else {
            return Err(DbError::InvalidSchemaData)?;
        };
        Ok(schema)
    }
}

#[derive(Debug)]
pub struct SchemaModel {
    pub name: String,
    pub size: u64,
    pub fields: Vec<(String, Schema)>,
}

impl PartialEq for SchemaModel {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }
        if self.fields.len() != other.fields.len() {
            return false;
        }

        for (i, (_, s)) in self.fields.iter().enumerate() {
            if *s != other.fields[i].1 {
                return false;
            }
        }

        true
    }
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

#[cfg(test)]
mod tests {
    use super::{Schema, SchemaModel};

    #[test]
    fn comp() {
        const NAMES: [&str; 9] = [
            "name 1", "name 2", "name 3", "name 4", "name 5", "name 6",
            "name 7", "name 8", "name 9",
        ];
        fn s(n: usize) -> String {
            NAMES[n.clamp(0, NAMES.len() - 1)].to_string()
        }

        assert_ne!(Schema::Gene, Schema::U8);
        assert_ne!(
            Schema::Gene,
            Schema::Array { length: 1, kind: Box::new(Schema::U8) }
        );
        assert_ne!(
            Schema::Array { length: 2, kind: Box::new(Schema::U16) },
            Schema::Array { length: 1, kind: Box::new(Schema::U16) }
        );
        assert_ne!(
            Schema::Array { length: 5, kind: Box::new(Schema::U16) },
            Schema::Array { length: 5, kind: Box::new(Schema::I64) }
        );
        assert_ne!(
            Schema::Gene,
            Schema::Model(SchemaModel { fields: vec![], size: 12, name: s(0) })
        );

        assert_ne!(
            Schema::Model(SchemaModel {
                name: s(2),
                size: 480,
                fields: vec![]
            }),
            Schema::Model(SchemaModel { name: s(3), size: 44, fields: vec![] }),
        );

        assert_ne!(
            Schema::Model(SchemaModel {
                name: s(2),
                size: 44,
                fields: vec![(s(0), Schema::U8), (s(0), Schema::U8)]
            }),
            Schema::Model(SchemaModel {
                name: s(3),
                size: 44,
                fields: vec![(s(5), Schema::U8)]
            }),
        );

        assert_ne!(
            Schema::Model(SchemaModel {
                name: s(2),
                size: 44,
                fields: vec![(s(0), Schema::U32)]
            }),
            Schema::Model(SchemaModel {
                name: s(3),
                size: 44,
                fields: vec![(s(5), Schema::U8)]
            }),
        );

        assert_eq!(Schema::Gene, Schema::Gene);
        assert_eq!(Schema::U8, Schema::U8);
        assert_eq!(Schema::I16, Schema::I16);

        assert_eq!(
            Schema::Array { length: 1, kind: Box::new(Schema::U16) },
            Schema::Array { length: 1, kind: Box::new(Schema::U16) }
        );
        assert_eq!(
            Schema::Model(SchemaModel { name: s(2), size: 44, fields: vec![] }),
            Schema::Model(SchemaModel { name: s(5), size: 44, fields: vec![] }),
        );

        assert_eq!(
            Schema::Model(SchemaModel {
                name: s(2),
                size: 44,
                fields: vec![(s(0), Schema::U8)]
            }),
            Schema::Model(SchemaModel {
                name: s(3),
                size: 44,
                fields: vec![(s(5), Schema::U8)]
            }),
        );
    }
}
