use crate::error::{DbError, ShahError};

#[cfg(target_arch = "x86_64")]
/// getrandom syscall
pub(crate) fn getrandom(buf: &mut [u8]) {
    unsafe {
        ::core::arch::asm! (
            "syscall",
            in("rax") 318,
            in("rdi") buf.as_ptr() as usize,
            in("rsi") buf.len(),
            in("rdx") 0
        );
    }
}

#[cfg(target_arch = "aarch64")]
/// getrandom syscall
pub(crate) fn getrandom(buf: &mut [u8]) {
    unsafe {
        ::core::arch::asm! (
            "svc 0",
            in("x8") 278,
            in("x0") buf.as_ptr() as usize,
            in("x1") buf.len(),
            in("x2") 0 // flags
        );
    }
}

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

pub trait Command {
    fn parse(args: std::env::Args) -> Self;
    fn help() -> String;
}

pub fn command<T: Command + Default>() -> T {
    let mut args = std::env::args();
    loop {
        let Some(arg) = args.next() else { break T::default() };
        if arg == "-c" {
            break T::parse(args);
        }
    }
}

pub trait AsUtf8Str {
    fn as_utf8_str(&self) -> &str;
}

impl AsUtf8Str for [u8] {
    fn as_utf8_str(&self) -> &str {
        match core::str::from_utf8(self) {
            Ok(v) => v,
            Err(e) => core::str::from_utf8(&self[..e.valid_up_to()])
                .unwrap_or_default(),
        }
    }
}
