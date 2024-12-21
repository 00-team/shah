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
