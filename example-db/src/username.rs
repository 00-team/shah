use shah::ShahError;
use shah::db::trie::TrieAbc;
use shah::models::Gene;

pub(crate) mod db {

    use shah::{
        SystemError,
        db::trie::{Trie, TrieKey},
    };

    use super::*;

    #[derive(Debug)]
    pub struct UsernameAbc;

    impl TrieAbc for UsernameAbc {
        type Item<'a> = &'a str;
        const ABC: &'static str = "abcdefghijklmnopqrstuvwxyz_0123456789";

        fn convert(&self, key: Self::Item<'_>) -> Result<TrieKey, ShahError> {
            if key.is_empty() {
                return Err(SystemError::TrieKeyEmpty)?;
            }

            fn cv(c: char) -> Result<usize, ShahError> {
                if c == '_' {
                    return Ok(26);
                }
                if c.is_ascii_digit() {
                    return Ok(((c as u8 - b'0') + 27) as usize);
                }
                if c.is_ascii_uppercase() {
                    return Ok((c as u8 - b'A') as usize);
                }
                if c.is_ascii_lowercase() {
                    return Ok((c as u8 - b'a') as usize);
                }
                Err(SystemError::BadTrieKey)?
            }

            let mut tk = TrieKey::new(key.len());
            let mut cs = key.chars();
            let Some(first) = cs.next() else {
                return Err(SystemError::BadTrieKey)?;
            };
            tk.root = cv(first)?;

            for ch in cs {
                tk.tree.push(cv(ch)?);
            }

            Ok(tk)
        }
    }

    // const PHONE_ABC: &str = "0123456789";
    pub type UsernameDb = Trie<{ UsernameAbc::ABC.len() }, UsernameAbc, Gene>;

    #[allow(dead_code)]
    pub(crate) fn setup() -> Result<UsernameDb, ShahError> {
        UsernameDb::new("username", UsernameAbc)
    }

    #[cfg(test)]
    mod tests {
        use shah::{
            ShahError,
            models::{Gene, GeneId},
        };

        use super::{UsernameAbc, UsernameDb};

        #[test]
        fn username_db() {
            let _ = std::fs::remove_file("data/tst.username.shah");
            let mut db = UsernameDb::new("tst.username", UsernameAbc).unwrap();

            let mock_data: &[(&str, usize, &[usize])] = &[
                ("007", 27, &[27, 34]),
                ("008", 27, &[27, 35]),
                ("018", 27, &[28, 35]),
                ("017", 27, &[28, 34]),
                // ("Sadra", 18, &[0, 3, 17, 0]),
                ("saDRA", 18, &[0, 3, 17, 0]),
                ("dr007cc", 3, &[17, 27, 27, 34, 2, 2]),
            ];

            for (i, (un, root, tree)) in mock_data.iter().enumerate() {
                let i = i as u64;
                let a = Gene { id: GeneId(i + 3), ..Default::default() };
                let b = Gene { id: GeneId((i + 3) * 2), ..Default::default() };

                let key = db.key(*un).expect("bad key");
                assert_eq!(key.root, *root);
                assert_eq!(&key.tree, tree);

                assert!(matches!(
                    db.get(&key).err().expect("first get"),
                    ShahError::NotFound(_)
                ));

                assert_eq!(db.set(&key, a).expect("set"), None);
                assert_eq!(db.get(&key).expect("get"), a);
                assert_eq!(db.set(&key, b).expect("set"), Some(a));
                assert_eq!(db.get(&key).expect("get"), b);
            }
        }
    }
}
