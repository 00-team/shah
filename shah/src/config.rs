use std::{io::ErrorKind, sync::OnceLock};

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
    pub data_dir: std::path::PathBuf,
}

impl ShahConfig {
    /// MAX_POS is 100TB
    pub const MAX_POS: u64 = 100 * 1024 * 1024 * 1024 * 1024;

    pub fn get() -> &'static Self {
        static STATE: OnceLock<ShahConfig> = OnceLock::new();

        let data_dir =
            std::env::var("SHAH_DATA_DIR").unwrap_or("data".to_string());

        let data_dir = std::path::Path::new(&data_dir);
        match std::fs::create_dir_all(data_dir) {
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => {
                panic!("could not create shah data dir: {data_dir:?}\n{e:#?}")
            }
            _ => {}
        }

        let server: u32 = eint!("SHAH_SERVER_INDEX", u32);
        if server == 0 {
            panic!("SHAH_SERVER_INDEX env must not be 0");
        }

        STATE.get_or_init(|| Self { server, data_dir: data_dir.into() })
    }
}
