pub mod db {
    use shah::db::pond::PondDb;
    use shah::models::Gene;
    use shah::{Duck, Entity, ShahError, ShahSchema};

    #[shah::model]
    #[derive(Debug, Entity, Duck, Clone, Copy, ShahSchema)]
    pub struct Note {
        #[entity(gene)]
        pub gene: Gene,
        pub user: Gene,
        pub pond: Gene,
        #[entity(growth)]
        pub growth: u64,
        #[entity(flags)]
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
