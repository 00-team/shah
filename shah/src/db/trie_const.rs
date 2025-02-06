use std::{
    fmt::Debug,
    fs::{create_dir_all, File},
    io::{Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::PathBuf,
    str::FromStr,
};

use crate::{models::Binary, NotFound, ShahError, SystemError};

pub trait TrieAbc {
    #[allow(clippy::result_unit_err)]
    fn convert_char(&self, c: char) -> Result<usize, ()>;
}

#[derive(Debug)]
pub struct TrieConst<
    const ABC_LEN: usize,
    const INDEX: usize,
    const CACHE: usize,
    Abc: TrieAbc,
    Val: Binary + Default + Copy,
> {
    pub abc: Abc,
    pub file: File,
    pub path: PathBuf,
    _val: PhantomData<Val>,
    cache_len: u64,
}

#[derive(Debug)]
pub struct TrieConstKey<const INDEX: usize> {
    pub cache: u64,
    pub index: [usize; INDEX],
}

impl<
        const ABC_LEN: usize,
        const INDEX: usize,
        const CACHE: usize,
        Abc,
        Val,
    > TrieConst<ABC_LEN, INDEX, CACHE, Abc, Val>
where
    Val: Binary + Default + Copy + Debug,
    Abc: TrieAbc,
{
    /// size of file position which is 8 byes
    const PS: u64 = core::mem::size_of::<u64>() as u64;

    const VALUE_SIZE: u64 = Val::N * ABC_LEN as u64;
    const NODE_SIZE: u64 = Self::PS * ABC_LEN as u64;
    const MAX_SIZE: u64 = if Self::VALUE_SIZE > Self::NODE_SIZE {
        Self::VALUE_SIZE
    } else {
        Self::NODE_SIZE
    };

    pub fn new(name: &str, abc: Abc) -> Self {
        assert!(CACHE > 0, "TrieConst CACHE must be at least 1");
        assert!(INDEX > 0, "TrieConst INDEX must be at least 1");

        create_dir_all("data/").expect("could not create the data directory");
        let path = PathBuf::from_str(&format!("data/{name}.trie-const.bin"))
            .unwrap_or_else(|_| {
                panic!("could not create a path with name: {name}")
            });

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .unwrap_or_else(|_| {
                panic!("could not open the database file: {path:?}")
            });

        Self {
            file,
            abc,
            _val: PhantomData::<Val>,
            path,
            cache_len: ABC_LEN.pow(CACHE as u32) as u64,
        }
    }

    pub fn setup(mut self) -> Result<Self, ShahError> {
        let db_size = self.db_size().expect("could not read db_size");
        let cache_size = self.cache_len * Self::PS;

        if db_size == 0 {
            self.file
                .seek(SeekFrom::Start(cache_size - 1))
                .expect("could not seek to the cache_size");

            self.file.write_all(&[0u8]).expect("could not write &[0u8]");

            return Ok(self);
        }

        assert!(db_size >= cache_size, "invalid trie-const caching");
        Ok(self)
    }

    pub fn db_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub fn convert_key(
        &self, key: &str,
    ) -> Result<TrieConstKey<INDEX>, ShahError> {
        assert_eq!(key.len(), CACHE + INDEX);

        let mut tkey = TrieConstKey::<INDEX> { cache: 0, index: [0; INDEX] };

        let cache_key = &key[0..CACHE];
        let index_key = &key[CACHE..];

        for (i, c) in cache_key.chars().rev().enumerate() {
            let Ok(x) = self.abc.convert_char(c) else {
                return Err(SystemError::BadTrieKey)?;
            };
            tkey.cache += (ABC_LEN.pow(i as u32) * x) as u64;
        }

        for (i, c) in index_key.chars().enumerate() {
            let Ok(x) = self.abc.convert_char(c) else {
                return Err(SystemError::BadTrieKey)?;
            };
            tkey.index[i] = x;
        }

        Ok(tkey)
    }

    pub fn get(&mut self, key: &TrieConstKey<INDEX>) -> Result<Val, ShahError> {
        let mut pos = key.cache * Self::PS;
        let mut node = [0u64; ABC_LEN];
        let mut node_value = [Val::default(); ABC_LEN];
        let db_size = self.db_size()?;

        if db_size < pos + Self::MAX_SIZE {
            return Err(NotFound::NoTrieValue)?;
        }

        self.file.seek(SeekFrom::Start(pos))?;
        self.file.read_exact(node[0].as_binary_mut())?;
        pos = node[0];
        if pos == 0 {
            return Err(NotFound::NoTrieValue)?;
        }

        for i in 0..INDEX {
            self.file.seek(SeekFrom::Start(pos))?;

            if i + 1 == INDEX {
                self.file.read_exact(node_value.as_binary_mut())?;
                return Ok(node_value[key.index[i]]);
            }

            self.file.read_exact(node.as_binary_mut())?;
            pos = node[key.index[i]];

            if pos == 0 || db_size < pos + Self::MAX_SIZE {
                return Err(NotFound::NoTrieValue)?;
            }
        }

        unreachable!()
    }

    pub fn set(
        &mut self, key: &TrieConstKey<INDEX>, val: Val,
    ) -> Result<Option<Val>, ShahError> {
        let mut pos = key.cache * Self::PS;
        let mut node = [0u64; ABC_LEN];
        let mut single = 0u64;
        let mut node_value = [Val::default(); ABC_LEN];
        let mut end_of_file = 0u64;
        let mut writing = false;
        let mut i = 0usize;

        self.file.seek(SeekFrom::Start(pos))?;
        self.file.read_exact(single.as_binary_mut())?;
        if single == 0 {
            end_of_file = self.db_size()?;
            single = end_of_file;
            self.file.seek(SeekFrom::Start(pos))?;
            self.file.write_all(single.as_binary())?;
            self.file.seek(SeekFrom::Start(end_of_file))?;
            writing = true;
        } else {
            pos = single;
        }

        if !writing {
            loop {
                let ki = key.index[i];
                self.file.seek(SeekFrom::Start(pos))?;

                if i + 1 == INDEX {
                    self.file.read_exact(node_value.as_binary_mut())?;
                    let old_value = node_value[ki];
                    node_value[ki] = val;
                    self.file.seek_relative(-(Self::VALUE_SIZE as i64))?;
                    self.file.write_all(node_value.as_binary())?;
                    return Ok(Some(old_value));
                }

                self.file.read_exact(node.as_binary_mut())?;

                i += 1;
                if node[ki] != 0 {
                    pos = node[ki];
                    continue;
                }

                end_of_file = self.db_size()?;
                node[ki] = end_of_file;

                self.file.seek(SeekFrom::Start(pos))?;
                self.file.write_all(node.as_binary())?;
                self.file.seek(SeekFrom::Start(end_of_file))?;
                break;
            }
        }

        // loop over ramaning links for writing them
        for n in i..INDEX {
            let ki = key.index[n];

            // every node after this will be written at the next block
            // so we just imagine that this will be the next link position
            end_of_file += Self::NODE_SIZE;

            // check if where are at the end of our links
            // then set the last value to user_id
            // if we are not set the value to the next link
            if n + 1 == INDEX {
                node_value.as_binary_mut().fill(0);
                node_value[ki] = val;
                self.file.write_all(node_value.as_binary())?;
            } else {
                node.as_binary_mut().fill(0);
                node[ki] = end_of_file;
                self.file.write_all(node.as_binary())?;
            }
        }

        Ok(None)
    }
}
