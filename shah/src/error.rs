use std::convert::Infallible;

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

    pub fn not_found<T: Into<u16>>(code: T) -> Self {
        Self { scope: 2, code: code.into() }
    }

    pub fn database<T: Into<u16>>(code: T) -> Self {
        Self { scope: 3, code: code.into() }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShahError {
    System(SystemError),
    NotFound(NotFound),
    Db(DbError),
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
            ShahError::Db(err) => Self::database(err),
        }
    }
}

impl From<NotFound> for ShahError {
    fn from(value: NotFound) -> Self {
        Self::NotFound(value)
    }
}

impl From<DbError> for ShahError {
    fn from(value: DbError) -> Self {
        Self::Db(value)
    }
}

impl From<SystemError> for ShahError {
    fn from(value: SystemError) -> Self {
        Self::System(value)
    }
}

impl From<Infallible> for ShahError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

// impl<T: Into<SystemError>> From<T> for ShahError {
//     fn from(value: T) -> Self {
//         Self::System(value.into())
//     }
// }

#[shah::enum_int(u16)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum NotFound {
    #[default]
    Unknown,
    GeneIdZero,
    GeneServerZero,
    GeneIterMismatch,
    GenePepperMismatch,
    OutOfBounds,
    IndexOutOfBounds,
    EntityNotAlive,
    SnakeIsFree,
    NoTrieValue,
    ListIdZero,
    EmptyItem,
    TriePosZero,
    ApexRootNotFound,
}

impl From<NotFound> for ErrorCode {
    fn from(value: NotFound) -> Self {
        Self::not_found(value as u16)
    }
}

#[shah::enum_int(u16)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DbError {
    #[default]
    Unknown,
    InvalidSchemaData,
    /// database name can only contain [a-Z] | - | [0-9]
    InvalidDbName,
    InvalidDbHead,
    InvalidDbSchema,
    InvalidDbMeta,
    InvalidDbContent,
    BadInit,
    NoDiskSpace,
    NoKoch,
}

#[shah::enum_int(u16)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SystemError {
    #[default]
    Unknown,
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
    GeneIdMismatch,
    /// using set for deleting aka seting alive to false without .del(...)
    DeadSet,
    SendTimeOut,
    PondNoEmptySlotWasFound,
    TrieKeyEmpty,
    BadCoords,
}

impl From<std::io::Error> for ShahError {
    fn from(value: std::io::Error) -> Self {
        log::warn!("IO {value:#?}");
        Self::System(SystemError::Io)
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
    Db(DbError),
    User(T),
}

impl<T: IsNotFound> ClientError<T> {
    pub fn not_found_ok(self) -> Result<(), ClientError<T>> {
        if self.is_not_found() { Ok(()) } else { Err(self) }
    }
}

impl<T: From<u16> + Copy> From<ErrorCode> for ClientError<T> {
    fn from(value: ErrorCode) -> Self {
        match value.scope {
            1 => ClientError::System(value.code.into()),
            2 => ClientError::NotFound(value.code.into()),
            3 => ClientError::Db(value.code.into()),
            127 => ClientError::User(value.code.into()),
            _ => ClientError::Unknown,
        }
    }
}

impl<T: From<u16> + Copy> From<ShahError> for ClientError<T> {
    fn from(value: ShahError) -> Self {
        match value {
            ShahError::Db(e) => ClientError::Db(e),
            ShahError::NotFound(e) => ClientError::NotFound(e),
            ShahError::System(e) => ClientError::System(e),
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
        matches!(self, Self::NotFound(_))
        // match self {
        //     Self::NotFound(_) => true,
        //     _ => false,
        // }
    }
}

pub trait OptNotFound<T, E: IsNotFound> {
    fn onf(self) -> Result<Option<T>, E>;
}

impl<T, E: IsNotFound> OptNotFound<T, E> for Result<T, E> {
    fn onf(self) -> Result<Option<T>, E> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(e) => {
                if e.is_not_found() {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }
}
