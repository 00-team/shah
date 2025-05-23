use super::TrieAbc;
use crate::models::{DbHead, ShahMagic, ShahMagicDb};
use crate::{AsUtf8Str, DbError, ShahError};

pub const TRIE_VERSION: u16 = 1;
pub const TRIE_MAGIC: ShahMagic =
    ShahMagic::new_const(ShahMagicDb::Trie as u16);

#[crate::model]
#[derive(Debug)]
pub struct TrieMeta {
    pub db: DbHead,
    pub abc_len: u64,
    pub abc: [u8; 4096],
}

impl TrieMeta {
    pub fn init<Abc: TrieAbc>(&mut self, name: &str) {
        self.db.init(TRIE_MAGIC, 0, name, TRIE_VERSION);
        let chars = Abc::ABC;
        self.abc_len = chars.len() as u64;
        self.abc = [0; 4096];
        self.abc[..chars.len()].clone_from_slice(chars.as_bytes());
    }

    pub fn check<Abc: TrieAbc>(&self, ls: &str) -> Result<(), ShahError> {
        self.db.check(ls, TRIE_MAGIC, 0, TRIE_VERSION)?;

        let abc = Abc::ABC;
        assert!(abc.len() < 4095);

        if self.abc_len != abc.len() as u64 {
            log::error!(
                "{ls} abc_len chaged. {} != {}",
                self.abc_len,
                abc.len()
            );
            return Err(DbError::InvalidDbMeta)?;
        }

        let abcb = abc.as_bytes();
        if &self.abc[..abcb.len()] != abcb {
            log::error!(
                "{ls} abc changed. {} != {abc}",
                self.abc.as_utf8_str_null_terminated()
            );
            return Err(DbError::InvalidDbMeta)?;
        }

        Ok(())
    }
}
