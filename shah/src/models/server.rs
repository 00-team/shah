use crate::REPLY_BODY_SIZE;

#[crate::model]
#[derive(Debug)]
pub struct OrderHead {
    pub size: u32,
    pub scope: u8,
    pub route: u8,
    _pad: [u8; 2],
    pub id: u64,
}

#[crate::model]
#[derive(Debug)]
pub struct ReplyHead {
    pub id: u64,
    pub size: u32,
    pub error: u32,
    pub elapsed: u64,
}

#[crate::model]
#[derive(Debug)]
pub struct Reply {
    pub head: ReplyHead,
    pub body: [u8; REPLY_BODY_SIZE],
}
