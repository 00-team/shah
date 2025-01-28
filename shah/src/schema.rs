use shah_macros::EnumCode;

#[derive(Debug, EnumCode)]
pub enum Schema {
    Model { name: String, size: u64, fields: Vec<Schema> },
    Array { length: u64, kind: Box<Schema> },
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
}

impl Schema {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output: Vec<u8> = vec![u16::from(self) as u8];
        match self {
            Self::Model { fields, size, .. } => {
                output.extend_from_slice(&(*size as u16).to_le_bytes());
                output.extend_from_slice(&(fields.len() as u16).to_le_bytes());
                fields.iter().for_each(|f| match f {
                    Self::Model { .. } | Self::Array { .. } => {
                        output.extend_from_slice(&f.to_bytes());
                    }
                    _ => {
                        output.push(u16::from(f) as u8);
                    }
                });
            }
            Self::Array { length, kind } => {
                output.extend_from_slice(&(*length as u32).to_le_bytes());
                match &(**kind) {
                    Self::Model { .. } | Self::Array { .. } => {
                        let mut v = kind.to_bytes();
                        output.append(&mut v);
                    }
                    t => {
                        output.push(u16::from(t) as u8);
                    }
                }
            }
            _ => {}
        }
        output
    }
}

pub trait ShahSchema {
    fn shah_schema() -> Schema;
}

macro_rules! impl_helper {
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

impl_helper! {
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
