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
        use super::{PhoneAbc, PhoneDb};
        use shah::Gene;

        #[test]
        fn phone_db() -> std::io::Result<()> {
            let db = PhoneDb::new("tests.phone", PhoneAbc);
            db.file.set_len(0)?;
            let mut db = db.setup();
            // return Ok(());

            let mut val = Gene { id: 12, ..Default::default() };

            let key = db.convert_key("223334044").unwrap();
            assert_eq!(key.cache, 22);
            assert_eq!(key.index, [3, 3, 3, 4, 0, 4, 4, 0, 0]);

            let res = db.get(&key);
            assert!(matches!(res, Ok(None)), "first get");

            let res = db.set(&key, val.clone());
            assert!(matches!(res, Ok(None)), "first set");

            let res = db.get(&key);
            assert!(res.is_ok(), "second get");
            let res = res.unwrap();
            assert!(res.is_some(), "second get");
            println!("res: {res:?}");

            val.id = 69;

            let old_val = db.set(&key, val.clone());
            println!("old_val: {old_val:?}");

            let res = db.get(&key);
            println!("res: {res:?}");

            // assert_eq!(index, [2, 2, 3, 3, 3, 4, 0, 4, 4]);

            Ok(())
        }
    }
}
