pub mod db {
    use shah::{db::pond::PondDb, error::SystemError, Gene};

    #[shah::model]
    #[derive(Debug, shah::Entity, Clone, Copy)]
    pub struct Note {
        pub gene: Gene,
        pub user: Gene,
        #[entity_flags]
        pub entity_flags: u8,
        pub note: [u8; 247],
    }

    pub type NoteDb = PondDb<Note>;

    #[allow(dead_code)]
    pub(crate) fn setup() -> Result<NoteDb, SystemError> {
        NoteDb::new("note")?.setup()
    }
}
