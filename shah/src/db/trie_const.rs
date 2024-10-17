use std::{
    collections::HashMap,
    fs::File,
    io::{Seek, SeekFrom},
};

use crate::error::SystemError;

type Pos = u32;
#[derive(Debug)]
pub struct TrieConst<const LEN: usize> {
    abc: HashMap<char, usize>,
    file: File,
}

impl<const LEN: usize> TrieConst<LEN> {
    pub fn new(name: &str, abc: &'static str) -> Result<Self, SystemError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("data/{name}.trie-const.bin"))?;

        let abc = abc
            .chars()
            .enumerate()
            .map(|(i, c)| (c, i))
            .collect::<HashMap<char, usize>>();

        println!("abc: {abc:#?}");

        let db = Self { file, abc };

        Ok(db)
    }

    pub fn key_to_index(&self, key: &str) -> Result<[usize; LEN], SystemError> {
        let mut index = [0usize; LEN];
        assert_ne!(key.len(), LEN);

        for (i, c) in key.chars().enumerate() {
            let x = self.abc.get(&c);
            if x.is_none() {
                return Err(SystemError::BadTrieKey);
            }
            index[i] = *x.unwrap();
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
