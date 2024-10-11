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
