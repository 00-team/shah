use std::array::TryFromSliceError;

// use shah_macros::model;

#[crate::model]
/// shah error for sending
#[derive(Debug)]
pub struct ErrorCode {
    scope: u16,
    pub code: u16,
}

impl ErrorCode {
    pub(crate) fn shah<T: Into<u16>>(code: T) -> Self {
        Self { scope: 1, code: code.into() }
    }

    pub fn user<T: Into<u16>>(code: T) -> Self {
        Self { scope: 2, code: code.into() }
    }

    pub fn as_u32(&self) -> u32 {
        ((self.code as u32) << 16) | self.scope as u32
    }
}

/// shah error code
#[derive(Debug)]
#[shah_macros::enum_code]
pub enum PlutusError {
    NotFound,
    Forbidden,
    RateLimited,
    Database,
    Io { reason: String },
    Args,
    ZeroGeneId,
    BadGenePepper,
    BadGeneIter,
    BadInputLength,
    BadApiIndex,
}

impl From<TryFromSliceError> for PlutusError {
    fn from(_: TryFromSliceError) -> Self {
        Self::Database
    }
}

impl From<std::io::Error> for PlutusError {
    fn from(value: std::io::Error) -> Self {
        Self::Io { reason: value.to_string() }
    }
}

impl From<PlutusError> for ErrorCode {
    fn from(value: PlutusError) -> Self {
        Self::shah(value)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::ErrorCode;
//
//     #[test]
//     fn test_error() {
//         // let err = ErrorCode::shah(69u16);
//         // let val = err.as_u32();
//         // let bin: [u8; ErrorCode::SIZE] = err.into();
//         //
//         // assert_eq!(bin, val.to_le_bytes());
//         // assert_eq!(bin, [1, 0, 69, 0]);
//     }
// }
