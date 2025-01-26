#[crate::model]
#[derive(Debug, Clone, Copy)]
pub struct ErrorCode {
    pub scope: u16,
    pub code: u16,
}

impl ErrorCode {
    pub fn system<T: Into<u16>>(code: T) -> Self {
        Self { scope: 1, code: code.into() }
    }

    pub fn not_found<T: Into<u16>>(code: T) -> Self {
        Self { scope: 2, code: code.into() }
    }

    pub fn user<T: Into<u16>>(code: T) -> Self {
        Self { scope: 127, code: code.into() }
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

#[derive(Debug, Clone, Copy)]
pub enum ShahError {
    System(SystemError),
    NotFound(NotFound),
}

impl ShahError {
    pub fn not_found_ok(self) -> Result<(), ShahError> {
        match self {
            ShahError::NotFound(_) => Ok(()),
            _ => Err(self),
        }
    }
}

impl From<ShahError> for ErrorCode {
    fn from(value: ShahError) -> Self {
        match value {
            ShahError::System(err) => Self::system(err),
            ShahError::NotFound(err) => Self::not_found(err),
        }
    }
}

impl From<NotFound> for ShahError {
    fn from(value: NotFound) -> Self {
        Self::NotFound(value)
    }
}

impl<T: Into<SystemError>> From<T> for ShahError {
    fn from(value: T) -> Self {
        Self::System(value.into())
    }
}

#[shah::enum_int(ty = u16)]
#[derive(Debug, Default, Clone, Copy)]
pub enum NotFound {
    #[default]
    Unknown = 0,
    ZeroGeneId,
    BadGeneIter,
    GeneIdNotInDatabase,
    EntityNotAlive,
    /// using set for deleting aka seting alive to false without .del(...)
    DeadSet,
    SnakeIsFree,
    BadGenePepper,
    NoTrieValue,
}

impl From<NotFound> for ErrorCode {
    fn from(value: NotFound) -> Self {
        Self::not_found(value as u16)
    }
}

#[shah::enum_int(ty = u16)]
#[derive(Debug, Default, Clone, Copy)]
pub enum SystemError {
    #[default]
    Unknown = 0,
    BadOrderId,
    Io,
    BadInputLength,
    BadApiIndex,
    BadTrieKey,
    SnakeCapacityIsZero,
    BadOffset,
    SnakeBadLength,
    /// could count parse gene from hex string
    GeneFromHexErr,
    /// this may happen if id of gene on the disk is not the correct id
    MismatchGeneId,
}

impl From<std::io::Error> for SystemError {
    fn from(value: std::io::Error) -> Self {
        log::warn!("IO {value:#?}");
        Self::Io
    }
}

impl From<SystemError> for ErrorCode {
    fn from(value: SystemError) -> Self {
        Self::system(value as u16)
    }
}

pub trait IsNotFound {
    fn is_not_found(&self) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub enum ClientError<T> {
    Unknown,
    System(SystemError),
    NotFound(NotFound),
    User(T),
}

impl<T: IsNotFound> ClientError<T> {
    pub fn not_found_ok(self) -> Result<(), ClientError<T>> {
        if self.is_not_found() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

impl<T: From<u16> + Copy> From<ErrorCode> for ClientError<T> {
    fn from(value: ErrorCode) -> Self {
        match value.scope {
            1 => ClientError::System(value.code.into()),
            2 => ClientError::NotFound(value.code.into()),
            127 => ClientError::User(value.code.into()),
            _ => ClientError::Unknown,
        }
    }
}

impl<T: IsNotFound> IsNotFound for ClientError<T> {
    fn is_not_found(&self) -> bool {
        match self {
            Self::NotFound(_) => true,
            Self::User(ue) => ue.is_not_found(),
            _ => false,
        }
    }
}

impl IsNotFound for ShahError {
    fn is_not_found(&self) -> bool {
        match self {
            Self::NotFound(_) => true,
            _ => false,
        }
    }
}
