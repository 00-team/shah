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
};
use xxhash_rust::xxh3;

pub trait KayakKey: ShahModel + Eq + Hash + ShahSchema {
    fn is_some(&self) -> bool;
    fn is_none(&self) -> bool;
    fn clear(&mut self);
}

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

pub struct WaterfallDb<
    Key: KayakKey,
    Val: ShahModel + ShahSchema,
    const LEN: usize,
> {
    meta: WaterfallMeta,
    keyak_map: Vec<Gene>,
    map_file: File,
    kayak: EntityDb<Kayak<Key, Val, LEN>>,
    tasks: TaskList<1, Task<Self>>,
}

impl<Key: KayakKey, Val: ShahModel + ShahSchema, const LEN: usize>
    WaterfallDb<Key, Val, LEN>
{
    pub fn new(path: &str) -> Result<Self, ShahError> {
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
            .open(data_path.join(format!("map.shah")))?;

        let mut db = Self {
            kayak: EntityDb::new(&format!("{path}/kayak"), 1)?,
            tasks: TaskList::new([Self::work_kayak]),
            meta: WaterfallMeta { level: 1, split: 0, count: 0 },
            keyak_map: Vec::new(),
            map_file,
        };

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        
        self.map_file.read;

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
        let cap = self.keyak_map.len() * LEN;
        let load = self.meta.count as f64 / cap as f64;
        if load < 0.75 {
            return Ok(());
        }

        let target = self.meta.split as usize;

        let kayak_gene = self.keyak_map[target];
        let mut key_val = Vec::with_capacity(LEN * 5);
        let mut kayak = Kayak::<Key, Val, LEN>::default();
        self.kayak.get(&kayak_gene, &mut kayak)?;

        loop {
            for rider in kayak.riders.iter_mut() {
                if rider.key.is_some() {
                    key_val.push((rider.key, rider.value));
                    rider.key.clear();
                }
            }

            if kayak_gene == kayak.gene {
                self.kayak.set(&mut kayak)?;
            }

            let overflow = kayak.overflow;
            if self.kayak.del(&overflow, &mut kayak).onf()?.is_none() {
                break;
            }
        }

        kayak.zeroed();
        kayak.index = target as u32 + 1 << self.meta.level;
        self.kayak.add(&mut kayak)?;
        assert_eq!(self.keyak_map.len() as u32, kayak.index);
        self.keyak_map.push(kayak.gene);

        self.meta.split += 1;
        if self.meta.split == 1 << self.meta.level {
            self.meta.level += 1;
            self.meta.split = 0;
        }

        for (key, val) in key_val {
            let v = self.inner_insert(key, val)?;
            assert!(v.is_none());
        }

        Ok(())
    }

    pub fn insert(
        &mut self, key: Key, value: Val,
    ) -> Result<Option<Val>, ShahError> {
        self.check_split()?;
        self.inner_insert(key, value)
    }

    fn inner_insert(
        &mut self, key: Key, value: Val,
    ) -> Result<Option<Val>, ShahError> {
        let hash = Self::hash_key(&key);
        let bucket = self.bucket(hash);

        let kayak_gene = &self.keyak_map[bucket];

        let mut kayak = Kayak::<Key, Val, LEN>::default();
        self.kayak.get(kayak_gene, &mut kayak)?;
        assert_eq!(bucket as u32, kayak.index);

        let new_rider = KayakRider { key, value, hash };

        loop {
            let mut empty_slot = None;
            for r in kayak.riders.iter_mut() {
                if r.hash == hash && r.key == key {
                    let old_value = Some(r.value);
                    r.value = value;
                    self.kayak.set(&mut kayak)?;
                    return Ok(old_value);
                }
                if empty_slot.is_none() && r.key.is_none() {
                    empty_slot = Some(r);
                }
            }

            if let Some(slot) = empty_slot {
                slot.clone_from(&new_rider);
                kayak.count += 1;
                self.kayak.set(&mut kayak)?;
                return Ok(None);
            }

            let mut parent = kayak;
            kayak.zeroed();
            kayak.count = 1;
            kayak.riders[0].clone_from(&new_rider);

            self.kayak.get_or_add(&parent.overflow, &mut kayak)?;

            if kayak.gene != parent.overflow {
                parent.overflow = kayak.gene;
                self.kayak.set(&mut parent)?;
                return Ok(None);
            }
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
