pub mod db {
    use shah::db::trie_const::TrieConst;

    pub type PhoneDb = TrieConst<9>;

    pub fn setup() -> PhoneDb {
        let db = PhoneDb::new("phone", "0123456789").expect("phone setup err");

        db
    }
}
