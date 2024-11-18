use std::{
    io::ErrorKind, os::unix::net::UnixDatagram, thread::sleep, time::Duration,
};

use crate::{Binary, ErrorCode, ReplyHead};

/// Order Taker
pub struct Taker {
    conn: UnixDatagram,
    reply: [u8; 1024 * 64],
    server: String,
}

impl Taker {
    pub fn init(server: &str, path: &str) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(path);
        let conn = UnixDatagram::bind(path)?;
        conn.set_read_timeout(Some(Duration::from_secs(5)))?;
        conn.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self { conn, server: server.to_string(), reply: [0u8; 1024 * 64] })
    }

    pub fn connect(&self) -> std::io::Result<()> {
        self.conn.connect(&self.server)?;
        Ok(())
    }

    pub fn reply_head(&self) -> &ReplyHead {
        ReplyHead::from_binary(&self.reply[0..ReplyHead::S])
    }

    pub fn reply_body(&self, size: usize) -> &[u8] {
        &self.reply[ReplyHead::S..ReplyHead::S + size]
    }

    pub fn take(&mut self, order: &[u8]) -> Result<(), ErrorCode> {
        self.reply[0..ReplyHead::S].fill(0);

        if let Err(e) = self.conn.send(order) {
            log::error!("send error: {e:#?}");
            match e.kind() {
                ErrorKind::NotConnected | ErrorKind::ConnectionRefused => {
                    for i in 0..3 {
                        log::info!("reconnect try: {i}");
                        if self.connect().is_ok() {
                            self.conn.send(order)?;
                            break;
                        }
                        sleep(Duration::from_secs(2));
                    }
                }
                _ => Err(e)?,
            }
        }
        self.conn.recv(&mut self.reply)?;

        // let (reply_head, reply_body) = reply.split_at(ReplyHead::S);
        let reply_head = self.reply_head();

        if reply_head.error != 0 {
            return Err(ErrorCode::from_u32(reply_head.error));
        }

        Ok(())
    }
}
