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
    pub type PhoneDb = TrieConst<10, 2, 7, PhoneAbc, Gene>;

    pub fn setup() -> PhoneDb {
        PhoneDb::new("phone", PhoneAbc).setup()
    }

    #[cfg(test)]
    mod tests {
        use super::PhoneAbc;
        use shah::{db::trie_const::TrieConst, Gene};
        type PhoneDb = TrieConst<10, 5, 4, PhoneAbc, Gene>;

        #[test]
        fn phone_db() {
            let db = PhoneDb::new("tests.phone", PhoneAbc);
            db.file.set_len(0).expect("file truncate");
            let mut db = db.setup();

            let mock_data = [
                ("223334044", 2233, [3, 4, 0, 4, 4]),
                ("183937071", 1839, [3, 7, 0, 7, 1]),
                ("192236504", 1922, [3, 6, 5, 0, 4]),
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

#[shah::api(api = crate::models::ExampleApi, scope = 1, error = crate::models::ExampleError)]
pub mod api {
    use shah::{ErrorCode, Gene};
    use crate::models::State;

    pub(crate) fn phone_add(
        state: &mut State, (phone, gene): (&[u8; 12], &Gene), _: (),
    ) -> Result<(), ErrorCode> {

        println!("phone_add: {phone:?} -> {gene:?}");

        Ok(())
    }
}
