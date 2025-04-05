use self::coords::IntoApexCoords;

use super::entity::EntityDb;
use crate::{
    config::ShahConfig, models::{Binary, Gene, GeneId}, utils, OptNotFound, ShahError, SystemError
};

mod coords;
pub use coords::ApexCoords;

#[derive(shah::ShahSchema)]
#[shah::model]
#[derive(Debug, Clone, Copy, shah::Entity)]
struct ApexTile<const S: usize> {
    gene: Gene,
    entity_flags: u8,
    growth: u64,
    level: u8, // 0 - 5 - 15
    _pad: [u8; 6],
    tiles: [Gene; S],
}

#[derive(Debug)]
pub struct ApexDb<
    const OFF: usize,
    const LVL: usize,
    const LEN: usize,
    const SIZ: usize,
> {
    tiles: EntityDb<ApexTile<SIZ>>,
}

impl<
        const OFF: usize,
        const LVL: usize,
        const LEN: usize,
        const SIZ: usize,
    > ApexDb<OFF, LVL, LEN, SIZ>
{

    const ROOT_ID: GeneId = GeneId(1);

    pub fn new(path: &str) -> Result<Self, ShahError> {
        assert!(LVL > 0, "LVL must be at least 1");
        assert!(LVL <= 5, "LVL must be at most 5");
        assert!(LEN > 0, "LEN must be at least 1");
        assert!(LVL * LEN < 30, "LVL * LEN must be at most 30");
        assert_eq!(
            1 << (LVL * 2),
            SIZ,
            "SIZ must be equal to: {}",
            1 << (LVL * 2)
        );

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

    pub fn get<Ac: IntoApexCoords>(
        &mut self, ac: Ac, data: &mut [Gene; SIZ],
    ) -> Result<usize, ShahError> {
        assert!(OFF == 0, "get can only be used with OFF of 0");
        let mut ac = ac.into()?;

        let mut tile = ApexTile::<SIZ>::default();
        self.tiles.get_id(Self::ROOT_ID, &mut tile)?;

        for i in 0..LEN {
            if ac.z <= LVL {
                let idx = ac.calc_index::<LVL>();
                let len = ac.calc_len::<LVL>();
                // let len: usize = 1 << ((LVL - z) * 2);
                // let idx = Self::calc_index(z, x, y);
                let src = &tile.tiles[idx..idx + len];
                data[..len].copy_from_slice(src);
                return Ok(len);
            }

            let idx = ac.split::<LVL>().calc_index::<LVL>();
            // z -= LVL;
            // let b: usize = 1 << z;
            // let idx = Self::calc_index(LVL, x / b, y / b);
            // x = x % b;
            // y = y % b;

            let gene = tile.tiles[idx];
            if i + 1 == LEN {
                data[0] = gene;
                return Ok(1);
            }
            self.tiles.get(&gene, &mut tile)?;
        }

        unreachable!()
    }

    pub fn set<Ac: IntoApexCoords>(
        &mut self, ac: Ac, value: &Gene,
    ) -> Result<(), ShahError> {
        // TODO: remove this
        assert!(OFF == 0, "for now");


        let mut ac = ac.into()?;

        if ac.z < LVL * LEN {
            log::error!(
                "when setting zoom must be at leat {}, provided zoom: {}",
                LVL * LEN,
                ac.z
            );
            return Err(SystemError::BadCoords)?;
        }

        let acc = ApexCoords {
            x: 0,
            y: 0,
            z: 0,
        };

        let mut tile = ApexTile::<SIZ>::default();
        if self.tiles.get_id(Self::ROOT_ID, &mut tile).onf()?.is_none() {
            tile.zeroed();
            tile.gene.id = Self::ROOT_ID;
            tile.gene.server = ShahConfig::get().server;
            utils::getrandom(&mut tile.gene.pepper);
            self.tiles.write_buf_at(&tile, Self::ROOT_ID)?;
        };

        // for i in 0..LEN {
        //     if ac.z <= LVL {
        //         let idx = ac.calc_index::<LVL>();
        //         let len = ac.calc_len::<LVL>();
        //         // let len: usize = 1 << ((LVL - z) * 2);
        //         // let idx = Self::calc_index(z, x, y);
        //         let src = &tile.tiles[idx..idx + len];
        //         data[..len].copy_from_slice(src);
        //         return Ok(len);
        //     }
        //
        //     let idx = ac.split::<LVL>().calc_index::<LVL>();
        //     // z -= LVL;
        //     // let b: usize = 1 << z;
        //     // let idx = Self::calc_index(LVL, x / b, y / b);
        //     // x = x % b;
        //     // y = y % b;
        //
        //     let gene = tile.tiles[idx];
        //     if i + 1 == LEN {
        //         data[0] = gene;
        //         return Ok(1);
        //     }
        //     self.tiles.get(&gene, &mut tile)?;
        // }

        Ok(())
    }

    pub fn get_from(
        &mut self, _root: &Gene, _zoom: u8, _x: u32, _y: u32,
        _data: &mut [Gene],
    ) -> Result<(), ShahError> {
        todo!("impl this")
    }
}
