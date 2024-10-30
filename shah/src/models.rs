use std::str::FromStr;

use crate::{error::SystemError, Binary};

pub type GeneId = u64;

#[crate::model]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Gene {
    pub id: GeneId,
    pub iter: u8,
    pub pepper: [u8; 3],
    pub server: u32,
}

impl FromStr for Gene {
    type Err = SystemError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != Gene::S * 2 {
            return Err(SystemError::GeneFromHexErr);
        }
        let mut gene = Gene::default();
        for (i, x) in gene.as_binary_mut().iter_mut().enumerate() {
            let Ok(b) = u8::from_str_radix(&s[i * 2..(i + 1) * 2], 16) else {
                return Err(SystemError::GeneFromHexErr);
            };
            *x = b;
        }
        Ok(gene)
    }
}

#[crate::model]
#[derive(Debug)]
pub struct OrderHead {
    pub size: u32,
    pub scope: u8,
    pub route: u8,
    _pad: [u8; 2],
}

#[crate::model]
#[derive(Debug)]
pub struct ReplyHead {
    pub size: u32,
    pub error: u32,
    pub elapsed: u64,
}
