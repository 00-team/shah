use std::sync::OnceLock;

macro_rules! evar {
    ($name:literal) => {
        std::env::var($name).expect(concat!($name, " was not found in env"))
    };
}

macro_rules! eint {
    ($name:literal, $ty:ident) => {
        evar!($name).parse::<$ty>().expect(concat!(
            $name,
            " env var must be a valid ",
            stringify!($ty)
        ))
    };
}

#[derive(Debug)]
/// Shah Config
pub struct ShahConfig {
    pub server: u32,
}

impl ShahConfig {
    /// MAX_POS is 100TB
    pub const MAX_POS: u64 = 100 * 1024 * 1024 * 1024 * 1024;

    pub fn get() -> &'static Self {
        static STATE: OnceLock<ShahConfig> = OnceLock::new();
        let server: u32 = eint!("SHAH_SERVER_INDEX", u32);
        if server == 0 {
            panic!("SHAH_SERVER_INDEX env must not be 0");
        }

        STATE.get_or_init(|| Self { server })
    }
}
