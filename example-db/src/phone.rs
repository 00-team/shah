pub mod db {
    use shah::db::trie_const::{TrieAbc, TrieConst};

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
    pub type PhoneDb = TrieConst<9, PhoneAbc>;

    pub fn setup() -> PhoneDb {
        PhoneDb::new("phone", PhoneAbc).expect("phone setup err")
    }

    #[cfg(test)]
    mod tests {
        use super::PhoneDb;
        use crate::phone::db::PhoneAbc;

        #[test]
        fn phone_db() {
            let db = match PhoneDb::new("tests.phone", PhoneAbc) {
                Ok(v) => v,
                Err(e) => {
                    panic!("tests.phone db setup: {e:#?}");
                }
            };

            let index = db.key_to_index("223334044").unwrap();
            assert_eq!(index, [2, 2, 3, 3, 3, 4, 0, 4, 4]);
        }
    }
}
