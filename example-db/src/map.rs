pub mod db {
    use shah::{db::apex::ApexDb, ShahError};

    #[derive(Debug)]
    pub struct MapDb {
        pub apex: ApexDb<6, 3, 4096>,
    }

    impl MapDb {
        pub fn new() -> Result<Self, ShahError> {
            let map = Self { apex: ApexDb::new("map")? };
            Ok(map)
        }
    }
}
