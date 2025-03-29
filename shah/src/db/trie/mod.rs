mod meta;

use crate::{models::Binary, OptNotFound};
use crate::{utils, NotFound, ShahError, SystemError};
use std::{
    fmt::Debug,
    io::{ErrorKind, Seek, SeekFrom},
    marker::PhantomData,
    os::unix::fs::FileExt,
    path::PathBuf,
};

pub use meta::*;

type Pos = u64;

pub trait TrieAbc {
    const ABC: &str;
    fn convert_char(&self, c: char) -> Option<usize>;
}

#[shah::model]
struct Node<const ABC_LEN: usize, Val: Binary + Default> {
    value: Val,
    child: [Pos; ABC_LEN],
}

#[derive(Debug)]
pub struct Trie<
    const ABC_LEN: usize,
    Abc: TrieAbc,
    Val: Binary + Default + Copy,
> {
    abc: Abc,
    file: std::fs::File,
    name: String,
    ls: String,
    _val: PhantomData<Val>,
}

#[derive(Debug)]
pub struct TrieKey {
    pub root: usize,
    pub tree: Vec<usize>,
}

impl TrieKey {
    fn new(capacity: usize) -> Self {
        Self { root: 0, tree: Vec::with_capacity(capacity) }
    }
}

impl<
        const ABC_LEN: usize,
        Abc: TrieAbc,
        Val: Binary + Default + Copy + Debug,
    > Trie<ABC_LEN, Abc, Val>
{
    pub fn new(name: &str, abc: Abc) -> Result<Self, ShahError> {
        assert_eq!(Abc::ABC.chars().count(), ABC_LEN, "invalid ABC_LEN");

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
            ls: format!("<Trie {name} />"),
            _val: PhantomData::<Val>,
        };

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        let mut meta = TrieMeta::default();
        if let Err(e) = self.read_at(&mut meta, 0) {
            e.not_found_ok()?;

            meta.init::<Abc>(&self.name);
            self.file.write_all_at(meta.as_binary(), 0)?;
        } else {
            meta.check::<Abc>(&self.ls)?;
        }

        let nn = Node::<ABC_LEN, Val>::N;

        if self.file_size()? < TrieMeta::N + nn {
            utils::falloc(&self.file, TrieMeta::N, nn)?;
        }

        Ok(())
    }

    pub fn file_size(&mut self) -> Result<u64, ShahError> {
        Ok(self.file.seek(SeekFrom::End(0))?)
    }

    pub fn key(&self, key: &str) -> Result<TrieKey, ShahError> {
        if key.is_empty() {
            return Err(SystemError::TrieKeyEmpty)?;
        }

        let mut tkey = TrieKey::new(key.len());

        for (i, ch) in key.chars().enumerate() {
            let Some(x) = self.abc.convert_char(ch) else {
                log::error!("{} convert_key: bad trie key", self.ls);
                return Err(SystemError::BadTrieKey)?;
            };

            assert!(
                x < ABC_LEN,
                "{} convert_key: x cannot be bigger than ABC_LEN",
                self.ls
            );

            if i == 0 {
                tkey.root = x;
                continue;
            }

            tkey.tree.push(x);
        }

        Ok(tkey)
    }

    fn read_at<T: Binary>(
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

    fn write_at(
        &mut self, node: &Node<ABC_LEN, Val>, pos: Pos,
    ) -> Result<(), ShahError> {
        self.file.write_all_at(node.as_binary(), pos)?;
        Ok(())
    }

    pub fn get(&mut self, key: &TrieKey) -> Result<Val, ShahError> {
        // let mut pos = TrieMeta::N + key.root * Self::PS;
        let mut node = Node::<ABC_LEN, Val>::default();

        self.read_at(&mut node, TrieMeta::N)?;
        let mut pos = node.child[key.root];
        if pos == 0 {
            return Err(NotFound::TriePosZero)?;
        }
        self.read_at(&mut node, pos)?;

        for x in key.tree.iter() {
            pos = node.child[*x];
            if pos == 0 {
                return Err(NotFound::TriePosZero)?;
            }
            self.read_at(&mut node, pos)?;
        }

        Ok(node.value)

        // // self.file.seek(SeekFrom::Start(pos))?;
        // // self.file.read_exact(node[0].as_binary_mut())?;
        // // pos = cache_value;
        // if pos == 0 {
        //     return Err(NotFound::NoTrieValue)?;
        // }
        //
        // for i in 0..INDEX {
        //     // self.file.seek(SeekFrom::Start(pos))?;
        //
        //     if i + 1 == INDEX {
        //         self.read_at(&mut node_value, pos)?;
        //         // self.file.read_exact(node_value.as_binary_mut())?;
        //         return Ok(node_value[key.index[i]]);
        //     }
        //
        //     self.read_at(&mut node, pos)?;
        //     // self.file.read_exact(node.as_binary_mut())?;
        //     pos = node[key.index[i]];
        //
        //     if pos == 0 {
        //         return Err(NotFound::NoTrieValue)?;
        //     }
        // }
        //
        // unreachable!()
    }

    fn add(&mut self, tree: &[usize], value: Val) -> Result<Pos, ShahError> {
        let mut child_pos = self.file.seek(SeekFrom::End(0))?;
        let mut node = Node::<ABC_LEN, Val> { value, ..Default::default() };
        self.write_at(&node, child_pos)?;

        for x in tree.iter().rev() {
            let curr_pos = self.file.seek(SeekFrom::End(0))?;
            node.zeroed();
            node.child[*x] = child_pos;
            self.write_at(&node, curr_pos)?;
            child_pos = curr_pos;
        }

        Ok(child_pos)
    }

    pub fn set(
        &mut self, key: &TrieKey, val: Val,
    ) -> Result<Option<Val>, ShahError> {
        let mut node = Node::<ABC_LEN, Val>::default();

        if self.read_at(&mut node, TrieMeta::N).onf()?.is_none() {
            node.zeroed();
            self.write_at(&node, TrieMeta::N)?;

            let pos = self.add(&key.tree, val)?;
            node.child[key.root] = pos;

            self.write_at(&node, TrieMeta::N)?;
            return Ok(None);
        }

        let mut pos = node.child[key.root];
        if pos == 0 || self.read_at(&mut node, pos).onf()?.is_none() {
            node.child[key.root] = self.add(&key.tree, val)?;
            self.write_at(&node, TrieMeta::N)?;
            return Ok(None);
        }

        for (i, x) in key.tree.iter().enumerate() {
            let cpos = node.child[*x];
            if cpos == 0 || self.read_at(&mut node, cpos).onf()?.is_none() {
                node.child[*x] = self.add(&key.tree[i + 1..], val)?;
                self.write_at(&node, pos)?;
                return Ok(None);
            }
            pos = cpos;
        }

        let old_value = node.value;
        node.value = val;
        self.write_at(&node, pos)?;
        Ok(Some(old_value))
    }
}
