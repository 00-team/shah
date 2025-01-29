use crate::error::SystemError;

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

pub(crate) fn validate_db_name(name: &str) -> Result<(), SystemError> {
    if name.len() < 1 || name.len() > 64 {
        return Err(SystemError::InvalidDbName);
    }
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' {
            return Err(SystemError::InvalidDbName);
        }
    }
    Ok(())
}
