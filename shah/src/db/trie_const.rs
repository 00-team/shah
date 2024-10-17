use std::{
    fmt::Debug,
    fs::{create_dir_all, File},
    io::{Seek, SeekFrom},
};

use crate::error::SystemError;

pub trait TrieAbc {
    #[allow(clippy::result_unit_err)]
    fn convert_char(&self, c: char) -> Result<usize, ()>;
}

type Pos = u32;
#[derive(Debug)]
pub struct TrieConst<const LEN: usize, T: Debug + TrieAbc> {
    abc: T,
    file: File,
}

impl<const LEN: usize, T: Debug + TrieAbc> TrieConst<LEN, T> {
    pub fn new(name: &str, abc: T) -> Result<Self, SystemError> {
        create_dir_all("data/")?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.trie-const.bin"))?;

        let db = Self { file, abc };

        Ok(db)
    }

    pub fn key_to_index(&self, key: &str) -> Result<[usize; LEN], SystemError> {
        let mut index = [0usize; LEN];
        let key = &key[..LEN];

        for (i, c) in key.chars().enumerate() {
            let Ok(x) = self.abc.convert_char(c) else {
                return Err(SystemError::BadTrieKey);
            };
            index[i] = x;
        }

        Ok(index)
    }

    // pub fn db_size(&mut self) -> std::io::Result<u64> {
    //     Ok(self.file.seek(SeekFrom::End(0))?)
    // }

    // pub fn setup(&mut self) -> Result<(), SystemError> {
    //     let db_size = self.db_size();
    //
    //     Ok(())
    // }
}
