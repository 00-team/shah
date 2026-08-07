use crate::{
    OptNotFound, ShahError, ShahModel,
    config::ShahConfig,
    db::entity::{EntityDb, EntityFlags},
    models::{
        Binary, Gene, Performed, ShahSchema, ShahString, Task, TaskList, Worker,
    },
};
use std::{
    fs::File,
    hash::{Hash, Hasher},
    io::{Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
};
use xxhash_rust::xxh3;

mod test;

pub trait KayakKey: ShahModel + Eq + Hash + ShahSchema {}

impl<T: ShahModel + Eq + Hash + ShahSchema> KayakKey for T {}

#[crate::model]
#[derive(Debug, crate::ShahSchema)]
pub struct KayakRider<Key: KayakKey, Val: ShahModel + ShahSchema> {
    pub hash: u64,
    pub key: Key,
    pub value: Val,
}

#[crate::model]
#[derive(Debug, crate::Entity, crate::ShahSchema)]
pub struct Kayak<Key: KayakKey, Val: ShahModel + ShahSchema, const LEN: usize> {
    pub gene: Gene,
    pub overflow: Gene,
    growth: u64,
    entity_flags: EntityFlags,
    _pad: [u8; 1],
    pub count: u16,
    /// only for buckets not for overflows, index of the this bucket in
    /// bucket map. only used when the bucket_map is gone
    pub index: u32,
    pub riders: [KayakRider<Key, Val>; LEN],
}

impl<K: KayakKey, V: ShahModel + ShahSchema, const L: usize> Kayak<K, V, L> {
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub const fn len(&self) -> usize {
        self.count as usize
    }
    pub fn riders(&self) -> &[KayakRider<K, V>] {
        &self.riders[..self.len()]
    }
}

#[crate::model]
#[derive(Debug, Hash, crate::ShahSchema)]
struct PhoneKey {
    cc: u16,
    phone: ShahString<12>,
}

// type XK = Kayak<PhoneKey, Gene, 100>;

#[crate::model]
#[derive(Debug)]
struct WaterfallMeta {
    level: u32,
    split: u32,
    count: u64,
}

#[derive(Debug)]
pub struct WaterfallDb<
    Key: KayakKey,
    Val: ShahModel + ShahSchema,
    const LEN: usize,
> {
    meta: WaterfallMeta,
    kayak_map: Vec<Gene>,
    map_file: File,
    kayak: EntityDb<Kayak<Key, Val, LEN>>,
    tasks: TaskList<1, Task<Self>>,
    init_level: u32,
}

impl<Key: KayakKey, Val: ShahModel + ShahSchema, const LEN: usize>
    WaterfallDb<Key, Val, LEN>
{
    pub fn new(path: &str, init_level: u32) -> Result<Self, ShahError> {
        let conf = ShahConfig::get();
        let data_path = conf.data_dir.join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path: {path}");

        crate::utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let map_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(data_path.join("map.shah"))?;

        let mut db = Self {
            init_level,
            kayak: EntityDb::new(&format!("{path}/kayak"), 1)?,
            tasks: TaskList::new([Self::work_kayak]),
            meta: WaterfallMeta { level: init_level, split: 0, count: 0 },
            kayak_map: Vec::new(),
            map_file,
        };

        db.init()?;

        Ok(db)
    }

    fn load_map(&mut self) -> Result<(), ShahError> {
        let mut meta = WaterfallMeta::default();
        self.map_file.read_exact_at(meta.as_binary_mut(), 0)?;

        let buckets = (1 << meta.level) + (meta.split) as usize;
        let mut kayak_map = vec![Gene::NONE; buckets];
        let (head, buf, tail) = unsafe { kayak_map.align_to_mut::<u8>() };
        assert!(head.is_empty() && tail.is_empty());

        self.map_file.read_exact_at(buf, WaterfallMeta::N)?;

        self.kayak_map = kayak_map;
        self.meta = meta;

        Ok(())
    }

    fn save_meta(&mut self) -> Result<(), ShahError> {
        self.map_file.write_all_at(self.meta.as_binary(), 0)?;
        Ok(())
    }

    fn save_map(&mut self) -> Result<(), ShahError> {
        self.save_meta()?;

        let (head, buf, tail) = unsafe { self.kayak_map.align_to_mut::<u8>() };
        assert!(head.is_empty() && tail.is_empty());

        self.map_file.write_all_at(buf, WaterfallMeta::N)?;

        Ok(())
    }

    fn init(&mut self) -> Result<(), ShahError> {
        if self.load_map().is_ok() {
            return Ok(());
        }

        if self.kayak.live.0 > 0 {
            todo!("the map is fucked. loop over all kayaks and build the map");
        }

        self.meta =
            WaterfallMeta { level: self.init_level, split: 0, count: 0 };
        let kc = 1 << self.meta.level;
        self.kayak_map = Vec::with_capacity(kc as usize);

        for i in 0..kc {
            let mut kayak = Kayak { index: i, ..Default::default() };
            self.kayak.add(&mut kayak)?;
            self.kayak_map.push(kayak.gene);
        }

        self.save_map()?;

        Ok(())
    }

    fn work_kayak(&mut self) -> Result<Performed, ShahError> {
        self.kayak.work()
    }

    fn hash_key(key: &Key) -> u64 {
        let mut hasher = xxh3::Xxh3::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn bucket(&self, hash: u64) -> usize {
        let n = 1 << self.meta.level;

        let mut b = hash & ((n - 1) as u64);

        if b < self.meta.split as u64 {
            b = hash & (((2 * n) - 1) as u64);
        }

        b as usize
    }

    fn check_split(&mut self) -> Result<(), ShahError> {
        let cap = self.kayak_map.len() * LEN;
        let load = self.meta.count as f64 / cap as f64;
        if load < 0.75 {
            return Ok(());
        }

        let target = self.meta.split as usize;

        let kayak_gene = self.kayak_map[target];
        let mut key_val = Vec::with_capacity(LEN * 5);
        let mut kayak = Kayak::<Key, Val, LEN>::default();
        self.kayak.get(&kayak_gene, &mut kayak)?;

        loop {
            for rider in kayak.riders() {
                key_val.push((rider.key, rider.value));
            }
            kayak.count = 0;

            let overflow = kayak.overflow;

            if kayak_gene == kayak.gene {
                kayak.overflow.clear();
                self.kayak.set(&mut kayak)?;
            }

            if self.kayak.del(&overflow, &mut kayak).onf()?.is_none() {
                break;
            }
        }

        kayak.zeroed();
        kayak.index = target as u32 + (1 << self.meta.level);
        self.kayak.add(&mut kayak)?;
        assert_eq!(self.kayak_map.len() as u32, kayak.index);
        self.kayak_map.push(kayak.gene);

        self.meta.split += 1;
        if self.meta.split == 1 << self.meta.level {
            self.meta.level += 1;
            self.meta.split = 0;
        }

        self.meta.count -= key_val.len() as u64;
        for (key, val) in key_val {
            let v = self.inner_insert(key, val)?;
            assert!(v.is_none());
        }

        let offset = self.map_file.seek(SeekFrom::End(0))?;
        let kms = (self.kayak_map.len() as u64 - 1) * Gene::N;
        if offset == WaterfallMeta::N + kms {
            self.map_file.write_all(kayak.gene.as_binary())?;
            self.save_meta()?;
        } else {
            unreachable!("should never happen basicaly");
            // self.save_map()?;
        }

        Ok(())
    }

    pub fn insert(
        &mut self, key: Key, value: Val,
    ) -> Result<Option<Val>, ShahError> {
        self.check_split()?;
        let old_count = self.meta.count;
        let v = self.inner_insert(key, value)?;
        if old_count != self.meta.count {
            self.save_meta()?;
        }
        Ok(v)
    }

    fn inner_insert(
        &mut self, key: Key, value: Val,
    ) -> Result<Option<Val>, ShahError> {
        let hash = Self::hash_key(&key);
        let bucket = self.bucket(hash);

        let kayak_gene = &self.kayak_map[bucket];

        let mut kayak = Kayak::<Key, Val, LEN>::default();
        self.kayak.get(kayak_gene, &mut kayak)?;
        assert_eq!(bucket as u32, kayak.index);

        let new_rider = KayakRider { key, value, hash };

        loop {
            let len = kayak.len();
            for r in kayak.riders[..len].iter_mut() {
                if r.hash == hash && r.key == key {
                    let old_value = Some(r.value);
                    r.value = value;
                    self.kayak.set(&mut kayak)?;
                    return Ok(old_value);
                }
            }

            if len < LEN {
                kayak.riders[len].clone_from(&new_rider);
                kayak.count += 1;
                self.kayak.set(&mut kayak)?;
                self.meta.count += 1;
                return Ok(None);
            }

            let mut parent = kayak;
            kayak.zeroed();
            kayak.count = 1;
            kayak.riders[0].clone_from(&new_rider);

            self.kayak.get_or_add(&parent.overflow, &mut kayak)?;

            if kayak.gene != parent.overflow {
                parent.overflow = kayak.gene;
                self.meta.count += 1;
                self.kayak.set(&mut parent)?;
                return Ok(None);
            }
        }
    }

    pub fn get(&mut self, key: &Key) -> Result<Val, ShahError> {
        let hash = Self::hash_key(key);
        let bucket = self.bucket(hash);

        let mut kayak_gene = self.kayak_map[bucket];
        let mut kayak = Kayak::<Key, Val, LEN>::default();

        loop {
            self.kayak.get(&kayak_gene, &mut kayak)?;
            for r in kayak.riders() {
                if r.hash == hash && &r.key == key {
                    return Ok(r.value);
                }
            }
            kayak_gene = kayak.overflow;
        }
    }

    pub fn del(&mut self, key: &Key) -> Result<Val, ShahError> {
        let hash = Self::hash_key(key);
        let bucket = self.bucket(hash);

        let mut kayak_gene = self.kayak_map[bucket];
        let mut kayak = Kayak::<Key, Val, LEN>::default();

        loop {
            self.kayak.get(&kayak_gene, &mut kayak)?;
            let mut idx = None;
            for (i, r) in kayak.riders().iter().enumerate() {
                if r.hash == hash && &r.key == key {
                    idx = Some(i);
                    break;
                }
            }

            if let Some(i) = idx {
                let value = kayak.riders[i].value;
                let len = kayak.count as usize;

                if i + 1 < len && len > 1 {
                    kayak.riders[i] = kayak.riders[len - 1];
                }

                // kayak.riders.copy_within(i + 1..kayak.count as usize, i);
                kayak.count -= 1;
                self.kayak.set(&mut kayak)?;
                self.meta.count -= 1;
                self.save_meta()?;
                return Ok(value);
            }

            kayak_gene = kayak.overflow;
        }
    }
}

impl<Key: KayakKey, Val: ShahModel + ShahSchema, const LEN: usize> Worker<1>
    for WaterfallDb<Key, Val, LEN>
{
    fn tasks(&mut self) -> &mut TaskList<1, Task<Self>> {
        &mut self.tasks
    }
}
