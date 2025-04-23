use std::sync::{Arc, atomic::AtomicBool};

pub const EXIT_SIGNALS: [i32; 5] =
    [libc::SIGPWR, libc::SIGABRT, libc::SIGTERM, libc::SIGQUIT, libc::SIGINT];

pub fn register_exit(flag: &Arc<AtomicBool>) -> std::io::Result<()> {
    for sig in EXIT_SIGNALS {
        signal_hook::flag::register(sig, flag.clone())?;
    }
    Ok(())
}
