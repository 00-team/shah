mod coords;

use super::entity::EntityDb;
use crate::{
    config::ShahConfig,
    db::entity::Entity,
    models::{Binary, Gene},
    utils, OptNotFound, ShahError,
};
use coords::{IntoApexCoords, MAX_ZOOM};

pub use coords::ApexCoords;

#[derive(shah::ShahSchema)]
#[shah::model]
#[derive(Debug, Clone, Copy, shah::Entity)]
struct ApexTile<const S: usize> {
    gene: Gene,
    growth: u64,
    entity_flags: u8,
    level: u8, // 0 - 6 - 12
    _pad: [u8; 6],
    tiles: [Gene; S],
}

#[derive(Debug)]
pub struct ApexDb<const LVL: usize, const LEN: usize, const SIZ: usize> {
    tiles: EntityDb<ApexTile<SIZ>>,
}

impl<const LVL: usize, const LEN: usize, const SIZ: usize>
    ApexDb<LVL, LEN, SIZ>
{
    pub fn new(path: &str) -> Result<Self, ShahError> {
        assert!(LVL > 0, "LVL must be at least 1");
        assert!(LVL <= 6, "LVL must be at most 6");
        assert!(LEN > 0, "LEN must be at least 1");
        assert!(LVL * LEN < MAX_ZOOM, "LVL * LEN must be at most {MAX_ZOOM}");
        assert_eq!(
            1 << (LVL * 2),
            SIZ,
            "SIZ must be equal to: {}",
            1 << (LVL * 2)
        );
        ApexTile::<SIZ>::__assert_padding();

        let data_path = std::path::Path::new("data/").join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let db = Self { tiles: EntityDb::new(&format!("{path}/apex"), 0)? };

        Ok(db)
    }

    pub fn get_value<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac,
    ) -> Result<Gene, ShahError> {
        let key = ac.into()?.full_key()?;

        let mut gene = *ShahConfig::apex_root();
        let mut tile = ApexTile::<SIZ>::default();

        for x in key.key().iter() {
            self.tiles.get(&gene, &mut tile)?;
            gene = tile.tiles[*x];
        }

        Ok(gene)
    }

    pub fn get_display<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac, data: &mut [u8; SIZ],
    ) -> Result<usize, ShahError> {
        let key = ac.into()?.display_key();

        let mut gene = *ShahConfig::apex_root();
        let mut tile = ApexTile::<SIZ>::default();

        for x in key.key().iter() {
            if self.tiles.get(&gene, &mut tile).onf()?.is_none() {
                return Ok(0);
            }
            gene = tile.tiles[*x];
        }

        let (last, size) = (key.last(), key.size());
        let list = &tile.tiles[(last * size)..(last + 1) * size];

        data.fill(0);
        for (i, g) in list.iter().enumerate() {
            let (byte, bit) = (i / 8, i % 8);
            if g.is_some() {
                data[byte] |= 1 << bit;
            }
            // data[i] = g.is_some();
        }

        Ok(size)
    }

    fn add(&mut self, tree: &[usize], value: Gene) -> Result<Gene, ShahError> {
        let mut gene = value;
        for (i, x) in tree.iter().rev().enumerate() {
            let mut tile = ApexTile::<SIZ>::default();
            tile.tiles[*x] = gene;
            tile.level = ((LEN - i - 1) * LVL) as u8;
            self.tiles.add(&mut tile)?;
            gene = tile.gene;
        }
        Ok(gene)
    }

    pub fn set<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac, value: &Gene,
    ) -> Result<Option<Gene>, ShahError> {
        let key = ac.into()?.full_key()?;

        let voiding = value.is_none();
        let apex_root = ShahConfig::apex_root();
        let mut parent = ApexTile::<SIZ>::default();
        let mut curnet = ApexTile::<SIZ>::default();

        if self.tiles.get(apex_root, &mut parent).onf()?.is_none() {
            parent.zeroed();
            parent.level = 0;
            parent.gene = *apex_root;
            parent.set_alive(true);
            self.tiles.set_unchecked(&mut parent)?;

            if !voiding {
                parent.tiles[key.root()] = self.add(key.tree(), *value)?;
                self.tiles.set_unchecked(&mut parent)?;
            }
            return Ok(None);
        };

        let keykey = key.key();
        for (i, x) in keykey[..keykey.len() - 1].iter().enumerate() {
            let gene = parent.tiles[*x];
            if self.tiles.get(&gene, &mut curnet).onf()?.is_none() {
                if !voiding {
                    parent.tiles[*x] = self.add(&keykey[i + 1..], *value)?;
                    self.tiles.set_unchecked(&mut parent)?;
                }
                return Ok(None);
            }
            parent = curnet;
        }

        let old_value = parent.tiles[key.leaf()];
        parent.tiles[key.leaf()] = *value;
        // if voiding && !parent.tiles.iter().any(|g| g.is_some()) {
        //     self.tiles.del_unchecked(&mut parent)
        // }
        self.tiles.set_unchecked(&mut parent)?;

        Ok(Some(old_value))
    }
}
