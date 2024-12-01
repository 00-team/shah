use crate::{Api, Binary, OrderHead, Reply, ReplyHead};
use std::{
    fmt::Debug,
    io::{self, ErrorKind},
    os::unix::net::{SocketAddr, UnixDatagram},
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
    let mut reply = Reply::default();

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

        reply.head.route = order_head.route;
        reply.head.scope = order_head.scope;

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

        // let (reply_head, reply_body) = reply.split_at_mut(ReplyHead::S);
        // let reply_head = ReplyHead::from_binary_mut(reply_head);
        // let reply_body = &mut reply_body[..route.output_size];

        let result = (route.caller)(state, order_body, &mut reply.body);
        reply.head.elapsed = time.elapsed().as_micros() as u64;

        match result {
            Ok(output_size) => {
                reply.head.error = 0;
                reply.head.size = output_size as u32;
                send(
                    &server,
                    &reply.as_binary()[..ReplyHead::S + output_size],
                    &addr,
                );
                // if send(&server, reply.head.as_binary(), &addr) {
                //     continue;
                // };
                // send(&server, &reply.body[..output_size], &addr);
            }
            Err(e) => {
                reply.head.error = e.as_u32();
                reply.head.size = 0;
                send(&server, reply.head.as_binary(), &addr);
            }
        }
    }
}

fn send(conn: &UnixDatagram, data: &[u8], addr: &SocketAddr) -> bool {
    if let Err(e) = conn.send_to_addr(data, addr) {
        log::error!("error: {e}");
        return true;
    }
    false
}
