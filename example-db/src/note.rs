pub mod db {
    use shah::db::pond::PondDb;
    use shah::models::Gene;
    use shah::{Duck, Entity, ShahError, ShahSchema};

    #[shah::model]
    #[derive(Debug, Entity, Duck, Clone, Copy, ShahSchema)]
    pub struct Note {
        pub gene: Gene,
        pub user: Gene,
        pub pond: Gene,
        pub growth: u64,
        pub entity_flags: u8,
        #[str]
        pub note: [u8; 247],
    }

    pub type NoteDb = PondDb<Note>;

    #[allow(dead_code)]
    pub(crate) fn init() -> Result<NoteDb, ShahError> {
        NoteDb::new("note", 1)
    }
}
