use crate::models::{ExampleError, State};
use shah::db::trie_const::TrieConst;
use shah::models::Gene;
use shah::{ErrorCode, ShahError};
use shah::{SystemError, db::trie_const::TrieConstAbc};

pub(crate) mod db {

    use super::*;

    #[derive(Debug)]
    pub struct PhoneAbc;

    impl TrieConstAbc<9> for PhoneAbc {
        const ABC: &str = "0123456789";
        type Item<'a> = &'a str;

        fn convert(&self, key: Self::Item<'_>) -> Result<[usize; 9], ShahError> {
            let mut out = [0; 9];
            if key.chars().count() != 9 {
                return Err(SystemError::BadTrieKey)?;
            }

            for (i, ch) in key.chars().enumerate() {
                if !ch.is_ascii_digit() {
                    return Err(SystemError::BadTrieKey)?;
                }
                out[i] = (ch as u8 - b'0') as usize;
            }

            Ok(out)
        }
    }

    // const PHONE_ABC: &str = "0123456789";
    pub type PhoneDb =
        TrieConst<{ PhoneAbc::ABC.len() }, 2, 7, 9, PhoneAbc, Gene>;

    #[allow(dead_code)]
    pub(crate) fn setup() -> Result<PhoneDb, ShahError> {
        PhoneDb::new("phone", PhoneAbc)
    }

    #[cfg(test)]
    mod tests {
        use super::PhoneAbc;
        use shah::ShahError;
        use shah::db::trie_const::TrieConst;
        use shah::models::{Gene, GeneId};

        type PhoneDb = TrieConst<10, 2, 7, 9, PhoneAbc, Gene>;

        #[test]
        fn phone_db() {
            let _ = std::fs::remove_file("data/tests.phone.shah");
            let mut db = PhoneDb::new("tests.phone", PhoneAbc).unwrap();

            let mock_data = [
                ("223334044", 2233340, [4, 4]),
                ("183937071", 1839370, [7, 1]),
                ("192236504", 1922365, [0, 4]),
                ("961772969", 9617729, [6, 9]),
                ("961772970", 9617729, [7, 0]),
            ];

            for (i, (phone, cache, index)) in mock_data.iter().enumerate() {
                let i = i as u64;
                let a = Gene { id: GeneId(i + 3), ..Default::default() };
                let b = Gene { id: GeneId((i + 3) * 2), ..Default::default() };
                let k = db.key(phone).expect("convert key");
                assert_eq!(k.cache, *cache);
                assert_eq!(k.index, *index);

                assert!(matches!(
                    db.get(&k).err().expect("get"),
                    ShahError::NotFound(_)
                ));

                assert_eq!(db.set(&k, a).expect("set"), None);
                assert_eq!(db.get(&k).expect("get"), a);
                assert_eq!(db.set(&k, b).expect("set"), Some(a));
                assert_eq!(db.get(&k).expect("get"), b);
            }
        }
    }
}

#[shah::api(scope = 1, error = crate::models::ExampleError)]
mod eapi {
    use super::*;

    pub(crate) fn phone_add(
        state: &mut State, inp: (&[u8; 12], &Gene),
        out: (&mut [u8; 12], &mut Gene),
    ) -> Result<(), ErrorCode> {
        println!("phone_add: {inp:#?}");

        let Ok(phone) = core::str::from_utf8(&inp.0[..11]) else {
            return Err(ExampleError::BadStr)?;
        };
        let key = state.phone.key(&phone[2..11])?;

        let old = state.phone.set(&key, *inp.1)?.unwrap_or_default();

        out.0.copy_from_slice(inp.0);
        out.1.clone_from(&old);

        Ok(())
    }
}
