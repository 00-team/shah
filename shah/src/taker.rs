use std::{
    io::ErrorKind, os::unix::net::UnixDatagram, sync::Mutex, thread::sleep,
    time::Duration,
};

use crate::{error::SystemError, Binary, ErrorCode, OrderHead, Reply};

/// Order Taker
pub struct Taker {
    conn: UnixDatagram,
    // reply: [u8; 1024 * 64],
    server: String,
    count: Mutex<u64>,
}

impl Taker {
    pub fn init(server: &str, path: &str) -> std::io::Result<Self> {
        let _ = std::fs::remove_file(path);
        let conn = UnixDatagram::bind(path)?;
        conn.set_read_timeout(Some(Duration::from_secs(5)))?;
        conn.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self { conn, server: server.to_string(), count: Mutex::new(0) })
    }

    pub fn connect(&self) -> std::io::Result<()> {
        self.conn.connect(&self.server)?;
        Ok(())
    }

    // pub fn reply_head(&self) -> &ReplyHead {
    //     ReplyHead::from_binary(&self.reply[0..ReplyHead::S])
    // }
    //
    // pub fn reply_body(&self, size: usize) -> &[u8] {
    //     &self.reply[ReplyHead::S..ReplyHead::S + size]
    // }

    pub fn take(&self, order: &mut [u8]) -> Result<Reply, ErrorCode> {
        let mut reply = Reply::default();
        // self.reply[0..ReplyHead::S].fill(0);

        let mut count = self.count.lock().unwrap();
        if *count == u64::MAX {
            *count = 0;
        }
        *count += 1;
        let order_head = OrderHead::from_binary_mut(order);
        order_head.id = *count;

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
        self.conn.recv(reply.as_binary_mut())?;

        if reply.head.id != *count {
            Err(SystemError::BadOrderId)?;
        }

        // let order_head = OrderHead::from_binary(order);
        // assert_eq!(reply.head.scope, order_head.scope);
        // assert_eq!(reply.head.route, order_head.route);

        if reply.head.error != 0 {
            return Err(ErrorCode::from_u32(reply.head.error));
        }

        // self.conn.recv(&mut reply.body)?;

        // let (reply_head, reply_body) = reply.split_at(ReplyHead::S);
        // let reply_head = self.reply_head();
        // let reply_head = ReplyHead::from_binary(&reply[0..ReplyHead::S]);
        // let reply_body = &reply[ReplyHead::S..ReplyHead::S + size];

        Ok(reply)
    }
}
