use crate::error::{DbError, ShahError};
use std::{fs::File, io, os::fd::AsRawFd};

pub trait AsStatic<T> {
    fn as_static(&mut self) -> &'static mut T;
}

impl<T> AsStatic<T> for T {
    fn as_static(&mut self) -> &'static mut T {
        unsafe { &mut *(self as *mut T) }

        // Convert the mutable reference to a raw pointer
        // let raw_ptr: *mut T = r;

        // Convert the raw pointer back to a mutable reference with 'static lifetime
        // &mut *raw_ptr
    }
}

pub(crate) fn getrandom(buf: &mut [u8]) {
    let ptr = buf.as_mut_ptr();
    unsafe { libc::getrandom(ptr as *mut libc::c_void, buf.len(), 0) };
}

pub(crate) fn falloc(file: &File, off: u64, len: u64) -> Result<(), ShahError> {
    let fd = file.as_raw_fd();
    let res = unsafe { libc::posix_fallocate64(fd, off as i64, len as i64) };
    if res == 0 {
        return Ok(());
    }

    macro_rules! err {
        ($kind:ident, $($msg:literal),*) => {
            Err(io::Error::new(io::ErrorKind::$kind, concat!($($msg),*)))?
        };
    }

    match res {
        libc::ENOSPC => Err(DbError::NoDiskSpace)?,

        libc::EBADF => err!(
            InvalidInput,
            "fd is not a valid file descriptor, or is not opened for writing."
        ),
        libc::ENODEV => {
            err!(InvalidInput, "fd does not refer to a regular file.")
        }
        libc::ESPIPE => err!(InvalidInput, "fd refers to a pipe."),
        libc::EINVAL => err!(
            InvalidInput,
            "offset was less than 0, or len was less than or equal to 0,",
            "or the underlying filesystem does not support the operation."
        ),

        libc::EINTR => {
            err!(Interrupted, "A signal was caught during execution.")
        }

        libc::EFBIG => {
            err!(Unsupported, "offset+len exceeds the maximum file size.")
        }
        libc::EOPNOTSUPP => err!(
            Unsupported,
            "The filesystem containing the file referred to by fd does not ",
            "support this operation. This error code can be returned by ",
            "C libraries that don't perform the emulation shown in CAVEATS, ",
            "such as musl libc."
        ),
        _ => Err(DbError::Unknown)?,
    }
}

// #[cfg(target_arch = "x86_64")]
// /// getrandom syscall
// pub(crate) fn getrandom(buf: &mut [u8]) {
//     unsafe {
//         ::core::arch::asm! (
//             "syscall",
//             in("rax") 318,
//             in("rdi") buf.as_ptr() as usize,
//             in("rsi") buf.len(),
//             in("rdx") 0
//         );
//     }
// }
//
// #[cfg(target_arch = "aarch64")]
// /// getrandom syscall
// pub(crate) fn getrandom(buf: &mut [u8]) {
//     unsafe {
//         ::core::arch::asm! (
//             "svc 0",
//             in("x8") 278,
//             in("x0") buf.as_ptr() as usize,
//             in("x1") buf.len(),
//             in("x2") 0 // flags
//         );
//     }
// }

pub(crate) fn validate_db_name(name: &str) -> Result<(), ShahError> {
    if name.is_empty() || name.len() > 64 {
        return Err(DbError::InvalidDbName)?;
    }
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' {
            return Err(DbError::InvalidDbName)?;
        }
    }
    Ok(())
}

pub(crate) const fn env_num(var: &'static str) -> u16 {
    let var = var.as_bytes();
    let len = var.len();
    let mut num = 0u16;

    assert!(len < 5, "bruv your version is 5 digit long. wtf");

    macro_rules! nth {
        ($($nth:literal),*) => {
            $(if len > $nth { num *= 10; num += (var[$nth] - b'0') as u16; })*
        };
    }

    nth!(0, 1, 2, 3);

    num
}
