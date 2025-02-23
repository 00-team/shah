use crate::models::{DbHead, ShahMagic, ShahMagicDb};
use crate::{AsUtf8Str, DbError, ShahError};

use super::TrieAbc;

pub const TRIE_CONST_VERSION: u16 = 1;
pub const TRIE_CONST_MAGIC: ShahMagic =
    ShahMagic::new_const(ShahMagicDb::TrieConst as u16);

#[crate::model]
pub struct TrieConstMeta {
    pub db: DbHead,
    pub index: u64,
    pub cache: u64,
    pub abc_len: u64,
    pub abc: [u8; 4096],
}

impl TrieConstMeta {
    pub fn init<Abc: TrieAbc>(
        &mut self, name: &str, index: usize, cache: usize,
    ) {
        let chars = Abc::chars();
        self.db.init(TRIE_CONST_MAGIC, 0, name, TRIE_CONST_VERSION);
        self.index = index as u64;
        self.cache = cache as u64;
        self.abc_len = chars.len() as u64;
        self.abc = [0; 4096];
        self.abc[..chars.len()].clone_from_slice(chars.as_bytes());
    }

    pub fn check<Abc: TrieAbc>(
        &self, ls: &str, index: usize, cache: usize,
    ) -> Result<(), ShahError> {
        self.db.check(ls, TRIE_CONST_MAGIC, 0, TRIE_CONST_VERSION)?;

        if self.index != index as u64 {
            log::error!("{ls} index value chaged. {} != {index}", self.index);
            return Err(DbError::InvalidDbMeta)?;
        }

        if self.cache != cache as u64 {
            log::error!("{ls} cache value chaged. {} != {cache}", self.cache);
            return Err(DbError::InvalidDbMeta)?;
        }

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
