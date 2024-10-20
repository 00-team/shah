#![allow(dead_code, unused)]

use crate::{Api, Binary, OrderHead, ReplyHead};
use std::{
    fmt::Debug,
    io::{self, ErrorKind},
    os::unix::net::UnixDatagram,
    time::{Duration, Instant},
};

pub fn run<T: Debug>(
    path: &str, state: &mut T, routes: &[&[Api<T>]],
) -> io::Result<()> {
    let _ = std::fs::remove_file(path);
    let server = UnixDatagram::bind(path)?;

    server.set_nonblocking(true)?;
    server.set_read_timeout(Some(Duration::from_secs(5)))?;
    server.set_write_timeout(Some(Duration::from_secs(5)))?;

    log::info!("plutus database starting\n{path}\n\n");

    let mut order = [0u8; 1024 * 64];
    let mut reply = [0u8; 1024 * 64];

    loop {
        let time = Instant::now();
        let (order_size, addr) = match server.recv_from(&mut order) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            e => e?,
        };

        let (order_head, order_body) = order.split_at(OrderHead::S);
        let order_head = OrderHead::from_binary(order_head);
        let order_body = &order_body[..order_size - OrderHead::S];

        let route = match routes
            .get(order_head.scope as usize)
            .and_then(|v| v.get(order_head.route as usize))
        {
            Some(v) => v,
            None => {
                log::warn!(
                    "invalid api index: [{}, {}]",
                    order_head.scope,
                    order_head.route
                );
                continue;
            }
        };

        log::debug!("route: {route:#?}");

        if route.input_size != order_body.len() {
            log::warn!(
                "invalid input size: {} != {}",
                order_body.len(),
                route.input_size,
            );
            continue;
        }

        let (reply_head, reply_body) = reply.split_at_mut(ReplyHead::S);
        let reply_head = ReplyHead::from_binary_mut(reply_head);
        // let reply_body = &mut reply_body[..route.output_size];

        match (route.caller)(state, order_body, reply_body) {
            Ok(output_size) => {
                reply_head.error = 0;
                reply_head.size = output_size as u32;
            }
            Err(e) => {
                reply_head.error = e.as_u32();
                reply_head.size = 0;
            }
        }

        reply_head.elapsed = time.elapsed().as_micros() as u64;
        let reply_size = ReplyHead::S + reply_head.size as usize;

        server.send_to_addr(&reply[..reply_size], &addr);
    }
}
