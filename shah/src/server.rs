use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::{SocketAddr, UnixDatagram};
use std::time::{Duration, Instant};

use crate::models::{Binary, OrderHead, Reply, ReplyHead, Scope, ShahState};
use crate::ShahError;

const ORDER_SIZE: usize = 1024 * 64;

pub fn run<T: ShahState>(
    path: &str, state: &mut T, routes: &[Scope<T>],
) -> Result<(), ShahError> {
    let _ = std::fs::remove_file(path);
    let server = UnixDatagram::bind(path)?;

    server.set_nonblocking(true)?;
    server.set_read_timeout(Some(Duration::from_secs(5)))?;
    server.set_write_timeout(Some(Duration::from_secs(5)))?;

    log::info!("plutus database starting\n{path}\n\n");

    let mut order = [0u8; ORDER_SIZE];
    let mut reply = Reply::default();
    let mut did_not_performed = 0u64;
    let mut wait = false;

    let epfd = epoll_init(&server)?;
    // log::debug!("epfd: {epfd}");

    let mut events = [libc::epoll_event { events: 0, u64: 0 }; 5];

    loop {
        // log::debug!(
        //     "wait: {wait_for_server} && {did_not_performed} > 10"
        // );
        if wait && did_not_performed > 10 {
            let num_events = unsafe {
                libc::epoll_wait(
                    epfd,
                    events.as_mut_ptr(),
                    events.len() as i32,
                    -1,
                )
            };
            if num_events == -1 {
                return Err(io::Error::last_os_error())?;
            }

            // NOTE: we really dont care about what events has happend
            // after anything happend just check the recv

            // let mut evs = String::from("[");
            // for e in events.iter() {
            //     let u = e.u64;
            //     let ee = e.events;
            //     evs.push_str(&format!("{{ events: {ee}, u64: {u} }}, "));
            // }
            // evs.pop();
            // evs.pop();
            // evs.push(']');
            // log::debug!(
            //     "fd: {} | events: {num_events} | {evs}",
            //     server.as_raw_fd()
            // );
            //
            // for e in events.iter().take(num_events as usize) {
            //     if e.events & libc::EPOLLIN != {}
            // }
            //
            // for i in 0..num_events as usize {
            //     if events[i].events & libc::EPOLLIN as u32 != 0 {
            //         let u = events[i].u64;
            //         log::debug!("pollin {i} | {u}");
            //     }
            // }
        }

        wait = handle_order(&server, &mut order, &mut reply, state, routes)?;

        match state.work() {
            Ok(p) => {
                if p.0 {
                    did_not_performed = 0;
                } else if did_not_performed < 20 {
                    did_not_performed += 1
                }
            }
            Err(e) => {
                did_not_performed = 0;
                log::error!("work failed: {e:#?}");
            }
        }
    }
}

fn handle_order<T>(
    server: &UnixDatagram, order: &mut [u8; ORDER_SIZE], reply: &mut Reply,
    state: &mut T, routes: &[Scope<T>],
) -> Result<bool, ShahError> {
    let (order_size, addr) = match server.recv_from(order) {
        Ok(v) => v,
        Err(e) => match e.kind() {
            io::ErrorKind::WouldBlock => return Ok(true),
            _ => {
                log::error!("raw_os_error: {:?}", e.raw_os_error());
                log::error!("e: {e:#?}");
                return Err(e)?;
            }
        },
    };

    let time = Instant::now();

    let (order_head, order_body) = order.split_at(OrderHead::S);
    let order_head = OrderHead::from_binary(order_head);
    let order_body = &order_body[..order_size - OrderHead::S];

    reply.head.id = order_head.id;
    let Some(scope) = routes.get(order_head.scope as usize) else {
        log::warn!("unkown scope: {}", order_head.scope);
        return Ok(true);
    };

    let Some(route) = scope.routes.get(order_head.route as usize) else {
        log::warn!("unkown route: {}", order_head.route);
        return Ok(true);
    };

    log::debug!("order {}::{}", scope.name, route.name);

    if route.input_size != order_body.len() {
        log::warn!(
            "invalid input size: {} != {}",
            order_body.len(),
            route.input_size,
        );
        return Ok(true);
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
                server,
                &reply.as_binary()[..ReplyHead::S + output_size],
                &addr,
            );
            log::debug!(
                "reply {}::{}: {} {}μs",
                scope.name,
                route.name,
                reply.head.size,
                reply.head.elapsed
            );
            // if send(&server, reply.head.as_binary(), &addr) {
            //     continue;
            // };
            // send(&server, &reply.body[..output_size], &addr);
        }
        Err(e) => {
            reply.head.error = e.as_u32();
            reply.head.size = 0;
            send(server, reply.head.as_binary(), &addr);
            log::debug!(
                "reply {}::{}: err({:x}) {}μs",
                scope.name,
                route.name,
                reply.head.error,
                reply.head.elapsed
            );
        }
    }

    Ok(true)
}

fn send(conn: &UnixDatagram, data: &[u8], addr: &SocketAddr) -> bool {
    if let Err(e) = conn.send_to_addr(data, addr) {
        log::error!("error: {e}");
        return true;
    }
    false
}

fn epoll_init(server: &UnixDatagram) -> Result<i32, ShahError> {
    let epfd = unsafe { libc::epoll_create1(0) };
    if epfd < 0 {
        return Err(io::Error::last_os_error())?;
    }

    let server_fd = server.as_raw_fd();
    let mut event = libc::epoll_event {
        events: (libc::EPOLLIN | libc::EPOLLET) as u32,
        u64: server_fd as u64,
    };

    let res = unsafe {
        libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, server_fd, &mut event)
    };
    if res == -1 {
        return Err(io::Error::last_os_error())?;
    }

    Ok(epfd)
}
