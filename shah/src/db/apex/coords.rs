use crate::{ShahError, SystemError};

#[non_exhaustive]
pub struct ApexCoords {
    pub z: usize,
    pub x: usize,
    pub y: usize,
}

impl ApexCoords {
    pub const MAX_ZOOM: usize = 22;
    pub fn new(z: usize, x: usize, y: usize) -> Result<Self, ShahError> {
        if z > Self::MAX_ZOOM {
            log::error!("max zoom is {}. zoom: {z}", Self::MAX_ZOOM);
            return Err(SystemError::BadCoords)?;
        }

        let max: usize = 1 << z;
        if x > max || y > max {
            log::error!("max x,y is {max} for zoom {z}. x: {x} | y {y}");
            return Err(SystemError::BadCoords)?;
        }

        Ok(Self { z, x, y })
    }

    pub fn calc_len<const LVL: usize>(&self) -> usize {
        1usize << ((LVL - self.z) * 2)
    }

    pub fn split<const LVL: usize>(&mut self) -> Self {
        let b: usize = 1 << (self.z - LVL);
        let old = Self { z: LVL, x: self.x / b, y: self.y / b };
        self.z -= LVL;
        self.x %= b;
        self.y %= b;

        old

        // z -= LVL;
        // let idx = Self::calc_index(LVL, x / b, y / b);
        // x = x % b;
        // y = y % b;
    }

    pub fn calc_index<const LVL: usize>(&self) -> usize {
        let mut index = 0;
        for z in 1..=self.z {
            // 1 << (3 - 1) == 4 ** 2 -> 16  * idx
            // 1 << (3 - 2) == 2 ** 2 -> 4   * idx
            // 1 << (3 - 3) == 1 ** 2 -> 1   * idx
            let b = 1usize << (LVL - z);
            let sq = b * b;
            match ((self.x / b) % 2, (self.y / b) % 2) {
                (0, 0) => continue,
                (0, 1) => index += sq,
                (1, 0) => index += sq * 2,
                (1, 1) => index += sq * 3,
                _ => unreachable!(),
            }
        }
        index
    }
}

pub trait IntoApexCoords {
    fn into(self) -> Result<ApexCoords, ShahError>;
}

impl IntoApexCoords for (u8, u32, u32) {
    fn into(self) -> Result<ApexCoords, ShahError> {
        ApexCoords::new(self.0 as usize, self.1 as usize, self.2 as usize)
    }
}
