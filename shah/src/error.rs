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

pub trait IsNotFound {
    fn is_not_found(&self) -> bool;
}

#[shah::enum_int(ty = u16)]
#[derive(Debug, Default, Clone, Copy)]
pub enum SystemError {
    #[default]
    Unknown = 0,
    BadOrderId,
    Database,
    Io,
    ZeroGeneId,
    BadGenePepper,
    BadGeneIter,
    BadInputLength,
    BadApiIndex,
    GeneIdNotInDatabase,
    EntityNotAlive,
    BadTrieKey,
    SnakeCapacityIsZero,
    SnakeIsFree,
    BadOffset,
    SnakeBadLength,
    GeneFromHexErr,
    /// using set for deleting aka seting alive to false without .del(...)
    DeadSet,
}

impl IsNotFound for SystemError {
    fn is_not_found(&self) -> bool {
        matches!(
            self,
            SystemError::ZeroGeneId
                | SystemError::BadGeneIter
                | SystemError::GeneIdNotInDatabase
                | SystemError::EntityNotAlive
                | SystemError::DeadSet
                | SystemError::SnakeIsFree
                | SystemError::BadGenePepper
        )
    }
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

#[derive(Debug, Clone, Copy)]
pub enum ClientError<T: Clone + Copy> {
    Unknown,
    System(SystemError),
    User(T),
}

impl<T: From<u16> + Copy> From<ErrorCode> for ClientError<T> {
    fn from(value: ErrorCode) -> Self {
        match value.scope {
            1 => ClientError::System(value.code.into()),
            2 => ClientError::User(value.code.into()),
            _ => ClientError::Unknown,
        }
    }
}

impl<T: IsNotFound + Copy> IsNotFound for ClientError<T> {
    fn is_not_found(&self) -> bool {
        match self {
            Self::Unknown => false,
            Self::User(u) => u.is_not_found(),
            Self::System(s) => s.is_not_found(),
        }
    }
}
