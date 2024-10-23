pub mod db {
    use shah::db::snake::SnakeDb;

    pub fn setup() -> SnakeDb {
        let db = SnakeDb::new("detail").expect("detail setup");
        db.setup().expect("detail setup")
    }
}
