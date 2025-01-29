pub mod db {
    use shah::{db::pond::PondDb, error::ShahError, Gene};

    #[shah::model]
    #[derive(Debug, shah::Entity, shah::Duck, Clone, Copy, shah::ShahSchema)]
    pub struct Note {
        pub gene: Gene,
        pub user: Gene,
        pub pond: Gene,
        #[entity_flags]
        pub entity_flags: u8,
        #[str]
        pub note: [u8; 247],
    }

    pub type NoteDb = PondDb<Note>;

    #[allow(dead_code)]
    pub(crate) fn setup() -> Result<NoteDb, ShahError> {
        NoteDb::new("note")?.setup()
    }
}
