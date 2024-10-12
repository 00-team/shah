pub type GeneId = u64;

#[crate::model]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Gene {
    pub id: GeneId,
    pub iter: u8,
    pub pepper: [u8; 3],
    pub server: u32,
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
