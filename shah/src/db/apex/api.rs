use super::{ApexDb, ApexTile, coords::IntoApexCoords};
use crate::{
    OptNotFound, ShahError,
    config::ShahConfig,
    db::entity::Entity,
    models::{Binary, Gene},
};

impl<const LVL: usize, const LEN: usize, const SIZ: usize>
    ApexDb<LVL, LEN, SIZ>
{
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

    pub fn void<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac,
    ) -> Result<Gene, ShahError> {
        let apex_root = ShahConfig::apex_root();
        let key = ac.into()?.full_key()?;
        let mut tile_tree = [ApexTile::<SIZ>::default(); LEN];
        self.tiles.get(apex_root, &mut tile_tree[0])?;

        for (i, x) in key.key_branch().iter().enumerate() {
            let gene = tile_tree[i].tiles[*x];
            if self.tiles.get(&gene, &mut tile_tree[i + 1]).onf()?.is_none() {
                tile_tree[i + 1].set_alive(false);
                break;
            }
        }

        let old_value = tile_tree[LEN - 1].tiles[key.leaf()];

        for (i, x) in key.key().iter().enumerate().rev() {
            let t = &mut tile_tree[i];
            if !t.is_alive() {
                continue;
            }
            t.tiles[*x].clear();
            if i != 0 || t.tiles.iter().any(|g| g.is_some()) {
                self.tiles.set(t)?;
                break;
            }
            let gene = t.gene;
            self.tiles.del(&gene, t).onf()?;
        }

        Ok(old_value)
    }

    pub fn mark<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac, value: &Gene,
    ) -> Result<Option<Gene>, ShahError> {
        assert!(value.is_some(), "use void api for voiding");
        let key = ac.into()?.full_key()?;

        let apex_root = ShahConfig::apex_root();
        let mut parent = ApexTile::<SIZ>::default();
        let mut curnet = ApexTile::<SIZ>::default();

        if self.tiles.get(apex_root, &mut parent).onf()?.is_none() {
            parent.zeroed();
            parent.level = 0;
            parent.gene = *apex_root;
            parent.set_alive(true);
            self.tiles.set_unchecked(&mut parent)?;

            parent.tiles[key.root()] = self.add(key.tree(), *value)?;
            self.tiles.set_unchecked(&mut parent)?;

            return Ok(None);
        };

        let keykey = key.key();
        for (i, x) in keykey[..keykey.len() - 1].iter().enumerate() {
            let gene = parent.tiles[*x];
            if self.tiles.get(&gene, &mut curnet).onf()?.is_none() {
                parent.tiles[*x] = self.add(&keykey[i + 1..], *value)?;
                self.tiles.set_unchecked(&mut parent)?;

                return Ok(None);
            }
            parent = curnet;
        }

        let old_value = parent.tiles[key.leaf()];
        parent.tiles[key.leaf()] = *value;
        self.tiles.set_unchecked(&mut parent)?;

        Ok(Some(old_value))
    }
}
