use crate::models::{DbHead, ShahMagic, ShahMagicDb};
use crate::{AsUtf8Str, DbError, ShahError};

use super::TrieAbc;

pub const TRIE_VERSION: u16 = 1;
pub const TRIE_MAGIC: ShahMagic =
    ShahMagic::new_const(ShahMagicDb::Trie as u16);

#[crate::model]
pub struct TrieMeta {
    pub db: DbHead,
    pub abc_len: u64,
    pub abc: [u8; 4096],
}

impl TrieMeta {
    pub fn init<Abc: TrieAbc>(&mut self, name: &str) {
        let chars = Abc::chars();
        self.db.init(TRIE_MAGIC, 0, name, TRIE_VERSION);
        self.abc_len = chars.len() as u64;
        self.abc = [0; 4096];
        self.abc[..chars.len()].clone_from_slice(chars.as_bytes());
    }

    pub fn check<Abc: TrieAbc>(&self, ls: &str) -> Result<(), ShahError> {
        self.db.check(ls, TRIE_MAGIC, 0, TRIE_VERSION)?;

        let chars = Abc::chars();
        assert!(chars.len() < 4096);

        if self.abc_len != chars.len() as u64 {
            log::error!(
                "{ls} abc_len chaged. {} != {}",
                self.abc_len,
                chars.len()
            );
            return Err(DbError::InvalidDbMeta)?;
        }

        let abc = chars.as_bytes();
        if &self.abc[..abc.len()] != abc {
            log::error!(
                "{ls} abc changed. {} != {chars}",
                self.abc.as_utf8_str_null_terminated()
            );
            return Err(DbError::InvalidDbMeta)?;
        }

        Ok(())
    }
}
