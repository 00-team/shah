pub mod db {
    use shah::{ShahError, db::apex::ApexDb, models::Gene};

    #[derive(Debug)]
    pub struct MapDb {
        #[allow(dead_code)]
        pub apex: ApexDb<6, 3, 4096, Gene>,
    }

    impl MapDb {
        #[allow(dead_code)]
        pub fn new() -> Result<Self, ShahError> {
            let map = Self { apex: ApexDb::new("map")? };
            Ok(map)
        }
    }
}
