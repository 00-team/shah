#[crate::model]
#[derive(Debug)]
pub struct ErrorCode {
    pub scope: u16,
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

#[derive(Debug)]
#[shah_macros::enum_code]
pub enum SystemError {
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

// impl From<TryFromSliceError> for SystemError {
//     fn from(_: TryFromSliceError) -> Self {
//         Self::Database
//     }
// }

impl From<std::io::Error> for SystemError {
    fn from(value: std::io::Error) -> Self {
        Self::Io { reason: value.to_string() }
    }
}

impl From<SystemError> for ErrorCode {
    fn from(value: SystemError) -> Self {
        Self::shah(value)
    }
}
