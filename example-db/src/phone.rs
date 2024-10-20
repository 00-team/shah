pub mod db {
    use shah::{
        db::trie_const::{TrieAbc, TrieConst},
        Gene,
    };

    #[derive(Debug)]
    pub struct PhoneAbc;

    impl TrieAbc for PhoneAbc {
        fn convert_char(&self, c: char) -> Result<usize, ()> {
            if !c.is_ascii_digit() {
                return Err(());
            }
            Ok((c as u8 - b'0') as usize)
        }
    }

    // const PHONE_ABC: &str = "0123456789";
    pub type PhoneDb = TrieConst<10, 9, 2, PhoneAbc, Gene>;

    pub fn setup() -> PhoneDb {
        PhoneDb::new("phone", PhoneAbc).setup()
    }

    #[cfg(test)]
    mod tests {
        use super::PhoneAbc;
        use shah::{db::trie_const::TrieConst, Gene};
        type PhoneDb = TrieConst<10, 9, 4, PhoneAbc, Gene>;

        #[test]
        fn phone_db() {
            let db = PhoneDb::new("tests.phone", PhoneAbc);
            db.file.set_len(0).expect("file truncate");
            let mut db = db.setup();

            let mock_data = [
                ("223334044", 2233, [3, 4, 0, 4, 4, 0, 0, 0, 0]),
                ("183937071", 1839, [3, 7, 0, 7, 1, 0, 0, 0, 0]),
                ("192236504", 1922, [3, 6, 5, 0, 4, 0, 0, 0, 0]),
            ];

            for (i, (phone, cache, index)) in mock_data.iter().enumerate() {
                let i = i as u64;
                let a = Gene { id: i + 3, ..Default::default() };
                let b = Gene { id: (i + 3) * 2, ..Default::default() };
                let k = db.convert_key(phone).expect("convert key");
                assert_eq!(k.cache, *cache);
                assert_eq!(k.index, *index);

                assert_eq!(db.get(&k).expect("get"), None);
                assert_eq!(db.set(&k, a).expect("set"), None);
                assert_eq!(db.get(&k).expect("get"), Some(a));
                assert_eq!(db.set(&k, b).expect("set"), Some(a));
                assert_eq!(db.get(&k).expect("get"), Some(b));
            }
        }
    }
}
