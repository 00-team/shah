use super::{ApexDb, ApexTile, ApexTileData, coords::IntoApexCoords};
use crate::{OptNotFound, ShahError, db::entity::Entity, models::Binary};

impl<const LVL: usize, const LEN: usize, const SIZ: usize, D: ApexTileData>
    ApexDb<LVL, LEN, SIZ, D>
{
    pub fn get_value<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac,
    ) -> Result<D, ShahError> {
        let key = ac.into()?.full_key()?;

        let mut data = D::new(self.root);
        let mut tile = ApexTile::<SIZ, D>::default();

        for x in key.key().iter() {
            self.tiles.get(data.gene(), &mut tile)?;
            data = tile.tiles[*x];
        }

        Ok(data)
    }

    pub fn get_display<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac, output: &mut [D; SIZ],
    ) -> Result<usize, ShahError> {
        let key = ac.into()?.display_key();

        let mut data = D::new(self.root);
        let mut tile = ApexTile::<SIZ, D>::default();

        for x in key.key().iter() {
            if self.tiles.get(data.gene(), &mut tile).onf()?.is_none() {
                return Ok(0);
            }
            data = tile.tiles[*x];
        }

        let (last, size) = (key.last(), key.size());
        let list = &tile.tiles[(last * size)..(last + 1) * size];

        // println!(
        //     "last: {last} | size: {size} | SIZ: {SIZ} | list.len: {}",
        //     list.len()
        // );

        // output.fill(0);
        output.zeroed();
        for (i, d) in list.iter().enumerate() {
            output[i] = *d;
            // let (byte, bit) = (i / 8, i % 8);
            // if g.is_some() {
            //     output[byte] |= 1 << bit;
            // }
            // data[i] = g.is_some();
        }

        Ok(size)
    }

    pub fn void<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac,
    ) -> Result<D, ShahError> {
        let key = ac.into()?.full_key()?;
        let mut tile_tree = [ApexTile::<SIZ, D>::default(); LEN];
        self.tiles.keyed(&self.root, &mut tile_tree[0])?;

        for (i, x) in key.key_branch().iter().enumerate() {
            let data = tile_tree[i].tiles[*x];
            if self
                .tiles
                .get(data.gene(), &mut tile_tree[i + 1])
                .onf()?
                .is_none()
            {
                tile_tree[i + 1].entity_flags_mut().set_is_alive(false);
                break;
            }
        }

        let old_value = tile_tree[LEN - 1].tiles[key.leaf()];

        for (i, x) in key.key().iter().enumerate().rev() {
            let t = &mut tile_tree[i];
            if !t.entity_flags().is_alive() {
                continue;
            }
            t.tiles[*x].clear();
            if i == 0 || t.tiles.iter().any(|g| g.is_some()) {
                self.tiles.set(t)?;
                break;
            }
            let gene = t.gene;
            self.tiles.del(&gene, t).onf()?;
        }

        Ok(old_value)
    }

    pub fn mark<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac, value: &D,
    ) -> Result<Option<D>, ShahError> {
        assert!(value.is_some(), "use void api for voiding");
        let key = ac.into()?.full_key()?;

        let mut parent = ApexTile::<SIZ, D>::default();
        let mut curnet = ApexTile::<SIZ, D>::default();

        self.tiles.keyed(&self.root, &mut parent)?;
        // if self.tiles.get(&Gene::ROOT, &mut parent).onf()?.is_none() {
        //     parent.zeroed();
        //     parent.level = 0;
        //     parent.gene = Gene::ROOT;
        //     parent.set_alive(true);
        //     self.tiles.set_unchecked(&mut parent)?;
        //
        //     parent.tiles[key.root()] = self.add(key.tree(), *value)?;
        //     self.tiles.set_unchecked(&mut parent)?;
        //
        //     return Ok(None);
        // };

        let keykey = key.key();
        for (i, x) in keykey[..keykey.len() - 1].iter().enumerate() {
            let data = parent.tiles[*x];
            if self.tiles.get(data.gene(), &mut curnet).onf()?.is_none() {
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

    /// mark the tile if not exists aka if its void
    pub fn mark_void<Ac: IntoApexCoords<LVL, LEN>>(
        &mut self, ac: Ac, value: &D,
    ) -> Result<bool, ShahError> {
        assert!(value.is_some(), "use void api for voiding");
        let key = ac.into()?.full_key()?;

        let mut parent = ApexTile::<SIZ, D>::default();
        let mut curnet = ApexTile::<SIZ, D>::default();

        self.tiles.keyed(&self.root, &mut parent)?;

        let keykey = key.key();
        for (i, x) in keykey[..keykey.len() - 1].iter().enumerate() {
            let data = parent.tiles[*x];
            if self.tiles.get(data.gene(), &mut curnet).onf()?.is_none() {
                parent.tiles[*x] = self.add(&keykey[i + 1..], *value)?;
                self.tiles.set_unchecked(&mut parent)?;

                return Ok(false);
            }
            parent = curnet;
        }

        let old_value = parent.tiles[key.leaf()];
        if old_value.is_some() {
            return Ok(true);
        }

        parent.tiles[key.leaf()] = *value;
        self.tiles.set_unchecked(&mut parent)?;

        Ok(false)
    }
}
