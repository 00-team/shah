#![allow(dead_code)]

pub mod db {
    use shah::{db::apex::ApexDb, ShahError};

    #[derive(Debug)]
    pub struct MapDb {
        pub apex: ApexDb<0, 5, 3, 1024>,
    }

    impl MapDb {
        pub fn new() -> Result<Self, ShahError> {
            let map = Self { apex: ApexDb::new("map")? };
            Ok(map)
        }
    }
}
