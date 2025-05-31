use std::fmt::Debug;
use std::io;
use std::marker::PhantomData;
use std::os::fd::AsRawFd;
use std::os::unix::net::{SocketAddr, UnixDatagram};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::models::{Binary, OrderHead, Reply, ReplyHead, Scope, ShahState};
use crate::{ClientError, ShahError};

const ORDER_SIZE: usize = 1024 * 64;

pub struct Server<
    's,
    'r,
    E: From<u16> + Copy + Debug,
    const TL: usize,
    State: ShahState<TL> + 'static,
> {
    #[allow(dead_code)]
    path: PathBuf,
    state: &'s mut State,
    routes: &'r [Scope<State>],
    _err: PhantomData<E>,
    epfd: i32,
    sock: UnixDatagram,
    exit: Arc<AtomicBool>,
}

impl<
    's,
    'r,
    const TL: usize,
    State: ShahState<TL> + 'static,
    E: From<u16> + Copy + Debug,
> Server<'s, 'r, E, TL, State>
{
    pub fn new<P: Into<PathBuf>>(
        path: P, state: &'s mut State, routes: &'r [Scope<State>], _err: E,
    ) -> Result<Self, ShahError> {
        let path: PathBuf = path.into();
        let _ = std::fs::remove_file(&path);
        let sock = UnixDatagram::bind(&path)?;

        sock.set_nonblocking(true)?;
        sock.set_read_timeout(Some(Duration::from_secs(5)))?;
        sock.set_write_timeout(Some(Duration::from_secs(5)))?;

        let exit = Arc::new(AtomicBool::new(false));
        crate::signals::register_exit(&exit)?;

        let server = Self {
            exit,
            path,
            state,
            routes,
            epfd: 0,
            sock,
            _err: PhantomData::<E>,
        }
        .epoll_init()?;

        Ok(server)
    }

    pub fn run(mut self) -> Result<(), ShahError> {
        let mut order = [0u8; ORDER_SIZE];
        let mut reply = Reply::default();
        let mut did_not_performed = 0u64;
        let mut wait = false;
        let mut events = [libc::epoll_event { events: 0, u64: 0 }; 5];

        loop {
            if self.exit.load(Ordering::Relaxed) {
                log::info!("exited");
                break Ok(());
            }

            if wait && did_not_performed > 10 {
                let num_events = unsafe {
                    libc::epoll_wait(
                        self.epfd,
                        events.as_mut_ptr(),
                        events.len() as i32,
                        -1,
                    )
                };
                if num_events == -1 {
                    let e = io::Error::last_os_error();
                    if matches!(e.kind(), io::ErrorKind::Interrupted) {
                        log::info!("exited while wating");
                        return Ok(());
                    }
                    log::error!("epoll: {e:?}");
                    return Err(e)?;
                }
            }

            wait = self.handle_order(&mut order, &mut reply)?;

            match self.state.work() {
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

    fn handle_order(
        &mut self, order: &mut [u8; ORDER_SIZE], reply: &mut Reply,
    ) -> Result<bool, ShahError> {
        let (order_size, addr) = match self.sock.recv_from(order) {
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
        let Some(scope) = self.routes.get(order_head.scope as usize) else {
            log::warn!("unkown scope: {}", order_head.scope);
            return Ok(true);
        };

        let Some(route) = scope.routes.get(order_head.route as usize) else {
            log::warn!("unkown route: {}", order_head.route);
            return Ok(true);
        };

        // log::debug!("order {}::{}", scope.name, route.name);

        if route.input_size != order_body.len() {
            log::warn!(
                "{}::{} invalid input size: {} != {}",
                scope.name,
                route.name,
                order_body.len(),
                route.input_size,
            );
            return Ok(true);
        }

        // let (reply_head, reply_body) = reply.split_at_mut(ReplyHead::S);
        // let reply_head = ReplyHead::from_binary_mut(reply_head);
        // let reply_body = &mut reply_body[..route.output_size];

        reply.body[..route.max_output_size].fill(0);
        let result = (route.caller)(self.state, order_body, &mut reply.body);
        reply.head.elapsed = time.elapsed().as_micros() as u64;

        match result {
            Ok(output_size) => {
                reply.head.error = 0;
                reply.head.size = output_size as u32;
                self.send(
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
                self.send(reply.head.as_binary(), &addr);
                let err = ClientError::<E>::from(e);
                log::error!(
                    "reply {}::{}: c({e:?}) e({err:?}) {}μs",
                    scope.name,
                    route.name,
                    reply.head.elapsed
                );
            }
        }

        Ok(true)
    }

    fn send(&self, data: &[u8], to: &SocketAddr) -> bool {
        if let Err(e) = self.sock.send_to_addr(data, to) {
            log::error!("error: {e}");
            return true;
        }
        false
    }

    fn epoll_init(mut self) -> Result<Self, ShahError> {
        let epfd = unsafe { libc::epoll_create1(0) };
        if epfd < 0 {
            return Err(io::Error::last_os_error())?;
        }

        let server_fd = self.sock.as_raw_fd();
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

        self.epfd = epfd;
        Ok(self)
    }
}
