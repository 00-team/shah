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
    pub fn get() -> &'static Self {
        static STATE: OnceLock<ShahConfig> = OnceLock::new();

        STATE.get_or_init(|| Self { server: eint!("SHAH_SERVER_INDEX", u32) })
    }
}
