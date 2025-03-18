mod meta;

use std::{
    fmt::Debug,
    io::{ErrorKind, Seek, SeekFrom, Write},
    marker::PhantomData,
    os::unix::fs::FileExt,
    path::PathBuf,
};

use crate::models::Binary;
use crate::{utils, NotFound, ShahError, SystemError};

pub use meta::*;

pub trait TrieAbc {
    fn convert_char(&self, c: char) -> Option<usize>;
    fn chars() -> &'static str;
}

#[derive(Debug)]
pub struct TrieConst<
    const ABC_LEN: usize,
    const INDEX: usize,
    const CACHE: usize,
    Abc: TrieAbc,
    Val: Binary + Default + Copy,
> {
    abc: Abc,
    file: std::fs::File,
    name: String,
    ls: String,
    cache_len: u64,
    _val: PhantomData<Val>,
}

#[derive(Debug)]
pub struct TrieConstKey<const INDEX: usize> {
    pub cache: u64,
    pub index: [usize; INDEX],
}

impl<const INDEX: usize> Default for TrieConstKey<INDEX> {
    fn default() -> Self {
        Self { cache: 0, index: [0; INDEX] }
    }
}

impl<
        const ABC_LEN: usize,
        const INDEX: usize,
        const CACHE: usize,
        Abc: TrieAbc,
        Val: Binary + Default + Copy + Debug,
    > TrieConst<ABC_LEN, INDEX, CACHE, Abc, Val>
{
    /// size of file position which is 8 byes
    const PS: u64 = core::mem::size_of::<u64>() as u64;

    // const VALUE_SIZE: u64 = Val::N * ABC_LEN as u64;
    const NODE_SIZE: u64 = Self::PS * ABC_LEN as u64;
    // const MAX_SIZE: u64 = if Self::VALUE_SIZE > Self::NODE_SIZE {
    //     Self::VALUE_SIZE
    // } else {
    //     Self::NODE_SIZE
    // };

    pub fn new(name: &str, abc: Abc) -> Result<Self, ShahError> {
        assert!(CACHE > 0, "TrieConst CACHE must be at least 1");
        assert!(INDEX > 0, "TrieConst INDEX must be at least 1");

        assert_eq!(Abc::chars().chars().count(), ABC_LEN, "invalid ABC_LEN");

        std::fs::create_dir_all("data/")?;
        let data_path = PathBuf::from(format!("data/{name}.shah"));

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&data_path)?;

        let mut db = Self {
            file,
            abc,
            name: name.to_string(),
            ls: format!("<TrieConst {name} />"),
            cache_len: ABC_LEN.pow(CACHE as u32) as u64,
            _val: PhantomData::<Val>,
        };

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        let mut meta = TrieConstMeta::default();
        if let Err(e) = self.read_at(&mut meta, 0) {
            e.not_found_ok()?;

            meta.init::<Abc>(&self.name, INDEX, CACHE);
            self.file.write_all_at(meta.as_binary(), 0)?;
        } else {
            meta.check::<Abc>(&self.ls, INDEX, CACHE)?;
        }

        let cache_size = self.cache_len * Self::PS;

        if self.file_size()? <= TrieConstMeta::N {
            utils::falloc(&self.file, TrieConstMeta::N, cache_size)?;
        }

        // you cant change the cache_size mid way
        assert!(
            self.file_size()? >= TrieConstMeta::N + cache_size,
            "invalid trie-const caching"
        );

        Ok(())
    }

    pub fn file_size(&mut self) -> Result<u64, ShahError> {
        Ok(self.file.seek(SeekFrom::End(0))?)
    }

    pub fn convert_key(
        &self, key: &str,
    ) -> Result<TrieConstKey<INDEX>, ShahError> {
        assert_eq!(key.len(), CACHE + INDEX);

        let mut tckey = TrieConstKey::<INDEX>::default();

        let cache_key = &key[0..CACHE];
        let index_key = &key[CACHE..];

        for (i, c) in cache_key.chars().rev().enumerate() {
            let Some(x) = self.abc.convert_char(c) else {
                log::error!("{} convert_key: bad trie key", self.ls);
                return Err(SystemError::BadTrieKey)?;
            };
            tckey.cache += (ABC_LEN.pow(i as u32) * x) as u64;
        }

        for (i, c) in index_key.chars().enumerate() {
            let Some(x) = self.abc.convert_char(c) else {
                log::error!("{} convert_key: bad trie key", self.ls);
                return Err(SystemError::BadTrieKey)?;
            };
            tckey.index[i] = x;
        }

        Ok(tckey)
    }

    pub fn read_at<T: Binary>(
        &mut self, item: &mut T, pos: u64,
    ) -> Result<(), ShahError> {
        match self.file.read_exact_at(item.as_binary_mut(), pos) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => Err(NotFound::OutOfBounds)?,
                _ => {
                    log::error!("{} read_at: {e:?}", self.ls);
                    Err(e)?
                }
            },
        }
    }

    pub fn get(&mut self, key: &TrieConstKey<INDEX>) -> Result<Val, ShahError> {
        let mut pos = TrieConstMeta::N + key.cache * Self::PS;
        let mut node = [0u64; ABC_LEN];
        let mut node_value = [Val::default(); ABC_LEN];

        // read the next position at the current pos
        let _cp = pos;
        self.read_at(&mut pos, _cp)?;

        // self.file.seek(SeekFrom::Start(pos))?;
        // self.file.read_exact(node[0].as_binary_mut())?;
        // pos = cache_value;
        if pos == 0 {
            return Err(NotFound::NoTrieValue)?;
        }

        for i in 0..INDEX {
            // self.file.seek(SeekFrom::Start(pos))?;

            if i + 1 == INDEX {
                self.read_at(&mut node_value, pos)?;
                // self.file.read_exact(node_value.as_binary_mut())?;
                return Ok(node_value[key.index[i]]);
            }

            self.read_at(&mut node, pos)?;
            // self.file.read_exact(node.as_binary_mut())?;
            pos = node[key.index[i]];

            if pos == 0 {
                return Err(NotFound::NoTrieValue)?;
            }
        }

        unreachable!()
    }

    pub fn set(
        &mut self, key: &TrieConstKey<INDEX>, val: Val,
    ) -> Result<Option<Val>, ShahError> {
        let mut pos = TrieConstMeta::N + key.cache * Self::PS;
        let mut node = [0u64; ABC_LEN];
        let mut single = 0u64;
        let mut node_value = [Val::default(); ABC_LEN];
        // this is just stupid. incorrect warning from rust?
        #[allow(unused_assignments)]
        let mut end_of_file = 0u64;
        let mut i = 0usize;

        // self.file.seek(SeekFrom::Start(pos))?;
        // self.file.read_exact(single.as_binary_mut())?;
        self.read_at(&mut single, pos)?;
        if single == 0 {
            end_of_file = self.file.seek(SeekFrom::End(0))?;
            single = end_of_file;
            // self.file.seek(SeekFrom::Start(pos))?;
            self.file.write_all_at(single.as_binary(), pos)?;
            self.file.seek(SeekFrom::Start(end_of_file))?;
        } else {
            pos = single;

            loop {
                let ki = key.index[i];
                // self.file.seek(SeekFrom::Start(pos))?;

                if i + 1 == INDEX {
                    self.read_at(&mut node_value, pos)?;
                    // self.file.read_exact(node_value.as_binary_mut())?;
                    let old_value = node_value[ki];
                    node_value[ki] = val;
                    // self.file.seek_relative(-(Self::VALUE_SIZE as i64))?;
                    self.file.write_all_at(node_value.as_binary(), pos)?;
                    return Ok(Some(old_value));
                }

                self.read_at(&mut node, pos)?;
                // self.file.read_exact(node.as_binary_mut())?;

                i += 1;
                if node[ki] != 0 {
                    pos = node[ki];
                    continue;
                }

                end_of_file = self.file.seek(SeekFrom::End(0))?;
                node[ki] = end_of_file;

                // self.file.seek(SeekFrom::Start(pos))?;
                self.file.write_all_at(node.as_binary(), pos)?;
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
