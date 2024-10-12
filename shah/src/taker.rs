use std::{os::unix::net::UnixDatagram, time::Duration};

use crate::{Binary, ErrorCode, ReplyHead};

/// Order Taker
pub struct Taker {
    conn: UnixDatagram,
    reply: [u8; 1024 * 64],
}

impl Taker {
    pub fn init(server: &str, path: &str) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(&path);
        let conn = UnixDatagram::bind(&path)?;
        conn.connect(server)?;
        conn.set_read_timeout(Some(Duration::from_secs(5)))?;
        conn.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self { conn, reply: [0u8; 1024 * 64] })
    }

    pub fn reply_head<'a>(&'a self) -> &'a ReplyHead {
        ReplyHead::from_binary(&self.reply[0..ReplyHead::S])
    }

    pub fn reply_body<'a>(&'a self, size: usize) -> &'a [u8] {
        &self.reply[ReplyHead::S..ReplyHead::S + size]
    }

    pub fn take(&mut self, order: &[u8]) -> Result<(), ErrorCode> {
        self.reply[0..ReplyHead::S].fill(0);

        self.conn.send(order)?;
        self.conn.recv(&mut self.reply)?;

        // let (reply_head, reply_body) = reply.split_at(ReplyHead::S);
        let reply_head = self.reply_head();

        if reply_head.error != 0 {
            return Err(ErrorCode::from_u32(reply_head.error));
        }

        Ok(())
    }
}
