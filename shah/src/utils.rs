use crate::error::{DbError, ShahError};

pub(crate) fn getrandom(buf: &mut [u8]) {
    let ptr = buf.as_mut_ptr();
    unsafe { libc::getrandom(ptr as *mut libc::c_void, buf.len(), 0) };
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
