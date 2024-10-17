use std::fs::File;

use crate::error::SystemError;

pub struct TrieConst {
    abc: &'static str,
    file: File,
}

impl TrieConst {
    pub fn new(name: &str, abc: &'static str) -> Result<Self, SystemError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.trie-const.bin"))?;

        let db = Self { file, abc };

        Ok(db)
    }
}
