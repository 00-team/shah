use std::fmt::Debug;

use crate::{ShahError, SystemError};

pub const MAX_ZOOM: usize = 22;

#[derive(Debug)]
pub struct ApexFullKey<const LEN: usize> {
    key: [usize; LEN],
}

impl<const LEN: usize> ApexFullKey<LEN> {
    fn new() -> Self {
        Self { key: [0; LEN] }
    }

    pub fn key(&self) -> &[usize; LEN] {
        &self.key
    }

    pub const fn root(&self) -> usize {
        self.key[0]
    }

    pub fn tree(&self) -> &[usize] {
        &self.key[1..]
    }

    pub fn tree_rest(&self, idx: usize) -> &[usize] {
        &self.key[idx + 1..]
    }

    /// tree[..tree.len() - 1]
    pub fn branch(&self) -> &[usize] {
        &self.key[1..self.key.len() - 1]
    }

    /// `key[key.len() - 1]`
    pub fn leaf(&self) -> usize {
        self.key[self.key.len() - 1]
    }
}

#[derive(Debug)]
pub struct ApexDisplayKey<const LEN: usize> {
    key: [usize; LEN],
    len: usize,
    size: usize,
}

impl<const LEN: usize> ApexDisplayKey<LEN> {
    fn new() -> Self {
        Self { key: [0; LEN], len: 0, size: 0 }
    }

    pub fn key(&self) -> &[usize] {
        &self.key[..self.len]
    }

    pub fn last(&self) -> usize {
        self.key[self.len - 1]
    }

    pub const fn size(&self) -> usize {
        self.size
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ApexCoords<const LVL: usize, const LEN: usize> {
    z: usize,
    x: usize,
    y: usize,
}

impl<const LVL: usize, const LEN: usize> ApexCoords<LVL, LEN> {
    pub const fn z(&self) -> usize {
        self.z
    }

    pub const fn x(&self) -> usize {
        self.x
    }

    pub const fn y(&self) -> usize {
        self.y
    }

    pub const fn zxy(&self) -> (usize, usize, usize) {
        (self.z, self.x, self.y)
    }

    pub fn new(z: usize, x: usize, y: usize) -> Result<Self, ShahError> {
        if z > MAX_ZOOM {
            log::error!("max zoom is {MAX_ZOOM}. your zoom: {z}");
            return Err(SystemError::BadCoords)?;
        }

        let max: usize = (1 << z) - 1;
        if x > max || y > max {
            log::error!("max x,y is {max} for zoom {z}. x: {x} | y {y}");
            return Err(SystemError::BadCoords)?;
        }

        Ok(Self { z, x, y })
    }

    pub const fn calc_len(&self) -> usize {
        1usize << ((LVL - self.z) * 2)
    }

    pub fn display_key(&self) -> ApexDisplayKey<LEN> {
        let mut key = ApexDisplayKey::new();

        let (mut z, mut x, mut y) = self.zxy();

        for slot in key.key.iter_mut() {
            key.len += 1;

            if z <= LVL {
                *slot = Self::index(z, x, y);
                key.size = 1 << ((LVL - z) * 2);
                if z == LVL && key.len != LEN {
                    key.len += 1;
                    key.size = 1 << (LVL * 2);
                }
                return key;
            }

            z -= LVL;
            let b: usize = 1 << z;
            *slot = Self::index(LVL, x / b, y / b);
            x %= b;
            y %= b;
        }

        key.size = 1;
        key
    }

    pub fn full_key(&self) -> Result<ApexFullKey<LEN>, ShahError> {
        if self.z < LVL * LEN {
            return Err(SystemError::BadCoords)?;
        }

        let mut key = ApexFullKey::new();

        let (mut z, mut x, mut y) = self.zxy();
        for slot in key.key.iter_mut() {
            z -= LVL;
            let b: usize = 1 << z;
            *slot = Self::index(LVL, x / b, y / b);
            x %= b;
            y %= b;
        }

        Ok(key)
    }

    // pub fn split(&mut self) -> Self {
    //     let b: usize = 1 << (self.z - LVL);
    //     let old = Self { z: LVL, x: self.x / b, y: self.y / b };
    //     self.z -= LVL;
    //     self.x %= b;
    //     self.y %= b;
    //
    //     old
    //
    //     // z -= LVL;
    //     // let idx = Self::calc_index(LVL, x / b, y / b);
    //     // x = x % b;
    //     // y = y % b;
    // }

    fn index(z: usize, x: usize, y: usize) -> usize {
        let mut index = 0;
        for cz in 1..=z {
            // 1 << (3 - 1) == 4 ** 2 -> 16  * idx
            // 1 << (3 - 2) == 2 ** 2 -> 4   * idx
            // 1 << (3 - 3) == 1 ** 2 -> 1   * idx
            let b = 1usize << (z - cz);
            let sq = b * b;
            match ((x / b) % 2, (y / b) % 2) {
                (0, 0) => continue,
                (1, 0) => index += sq,
                (0, 1) => index += sq * 2,
                (1, 1) => index += sq * 3,
                _ => unreachable!(),
            }
        }
        index
    }
}

pub trait IntoApexCoords<const LVL: usize, const LEN: usize>: Debug {
    fn into(self) -> Result<ApexCoords<LVL, LEN>, ShahError>;
}

impl<const LVL: usize, const LEN: usize> IntoApexCoords<LVL, LEN>
    for (u8, u32, u32)
{
    fn into(self) -> Result<ApexCoords<LVL, LEN>, ShahError> {
        ApexCoords::new(self.0 as usize, self.1 as usize, self.2 as usize)
    }
}
