#[crate::model]
#[derive(Debug)]
pub struct ErrorCode {
    pub scope: u16,
    pub code: u16,
}

impl ErrorCode {
    pub fn system<T: Into<u16>>(code: T) -> Self {
        Self { scope: 1, code: code.into() }
    }

    pub fn user<T: Into<u16>>(code: T) -> Self {
        Self { scope: 2, code: code.into() }
    }

    pub fn as_u32(&self) -> u32 {
        ((self.code as u32) << 16) | self.scope as u32
    }

    pub fn from_u32(err: u32) -> Self {
        Self { code: (err >> 16) as u16, scope: err as u16 }
    }
}

impl From<std::io::Error> for ErrorCode {
    fn from(value: std::io::Error) -> Self {
        log::warn!("client io error: {value}");
        Self::system(SystemError::Io as u16)
    }
}

#[derive(Debug)]
#[repr(u16)]
pub enum SystemError {
    NotFound = 0,
    Forbidden,
    RateLimited,
    Database,
    Io,
    Args,
    ZeroGeneId,
    BadGenePepper,
    BadGeneIter,
    BadInputLength,
    BadApiIndex,
    GeneIdNotInDatabase,
    EntityNotAlive,
    BadTrieKey,
    SeekOutOfRange,
    SnakeCapacityIsZero,
}

impl From<std::io::Error> for SystemError {
    fn from(value: std::io::Error) -> Self {
        println!("io error: {value} - {value:#?}");
        log::warn!("io error: {value} - {value:#?}");
        Self::Io
    }
}

impl From<SystemError> for ErrorCode {
    fn from(value: SystemError) -> Self {
        Self::system(value as u16)
    }
}

impl From<u16> for SystemError {
    fn from(value: u16) -> Self {
        unsafe { core::mem::transmute(value) }
    }
}

#[derive(Debug)]
pub enum ClientError<T> {
    Unknown,
    System(SystemError),
    User(T),
}

impl<T: From<u16>> From<ErrorCode> for ClientError<T> {
    fn from(value: ErrorCode) -> Self {
        match value.scope {
            1 => ClientError::System(value.code.into()),
            2 => ClientError::User(value.code.into()),
            _ => ClientError::Unknown,
        }
    }
}
