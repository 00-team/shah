// mod detail;
mod models;
// mod note;
mod phone;
mod user;

// use std::io::Write;

use std::{cell::RefCell, ops::DerefMut, rc::Rc};

use shah::{db::entity::EntityMigration, error::ShahError, Command};

// const SOCK_PATH: &str = "/tmp/shah.sock";

#[derive(Debug, Default, Command)]
enum Commands {
    #[default]
    Help,
    Run,
    Gg,
}

// pub fn detail_get(
//     state: &mut State, gene: &Gene,
// ) -> Result<(Vec<u8>, SnakeHead), ShahError> {
//     let mut head = SnakeHead::default();
//     let mut buf = [0u8; BLOCK_SIZE];
//     state.detail.read(gene, &mut head, 0, &mut buf)?;
//
//     let len = head.length.min(head.capacity);
//     let len = if len == 0 { head.capacity } else { len } as usize;
//     // let mut v = Vec::with_capacity(len);
//     // unsafe { v.set_len(len) };
//     let mut v = vec![0u8; len];
//
//     if len > BLOCK_SIZE {
//         v[..BLOCK_SIZE].copy_from_slice(&buf);
//         for i in 1..=(len / BLOCK_SIZE) {
//             let off = i * BLOCK_SIZE;
//             state.detail.read(gene, &mut head, off as u64, &mut buf)?;
//             v[off..(off + BLOCK_SIZE).min(len)]
//                 .copy_from_slice(&buf[..(len - off).min(BLOCK_SIZE)])
//         }
//     } else {
//         v.copy_from_slice(&buf[..len]);
//     }
//
//     Ok((v, head))
//     // Ok(v.as_utf8_str().to_string())
// }
//
// pub fn detail_set(
//     state: &mut State, gene: Option<Gene>, data: &[u8],
// ) -> Result<SnakeHead, ShahError> {
//     // assert!(data.len() <= BLOCK_SIZE);
//
//     let len = data.len().min(detail::DETAIL_MAX);
//     let mut snake: Option<SnakeHead> = None;
//     if let Some(old) = &gene {
//         let mut old_head = SnakeHead::default();
//         state.detail.index.get(old, &mut old_head)?;
//         if old_head.capacity >= len as u64 {
//             snake = Some(old_head);
//         } else {
//             state.detail.free(old)?;
//         }
//     }
//     if snake.is_none() {
//         let cap = (len + detail::DETAIL_BUF).min(detail::DETAIL_MAX) as u64;
//         let mut head = SnakeHead::default();
//         state.detail.alloc(cap, &mut head)?;
//         snake = Some(head);
//     }
//     let snake = snake.unwrap();
//     let mut rethead = SnakeHead::default();
//     state.detail.write(&snake.gene, &mut rethead, 0, &data[0..len])?;
//
//     for i in 0..=(len / BLOCK_SIZE) {
//         let off = i * BLOCK_SIZE;
//         if len < (off + BLOCK_SIZE) {
//             let mut write_buffer = [0u8; BLOCK_SIZE];
//             let wlen = len - off;
//             write_buffer[0..wlen].copy_from_slice(&data[off..len]);
//             state.detail.write(
//                 &snake.gene,
//                 &mut rethead,
//                 off as u64,
//                 &write_buffer[0..wlen],
//             )?;
//         } else {
//             state.detail.write(
//                 &snake.gene,
//                 &mut rethead,
//                 off as u64,
//                 &data[off..off + BLOCK_SIZE],
//             )?;
//         }
//     }
//
//     state.detail.set_length(&snake.gene, &mut rethead, len as u64)?;
//
//     Ok(rethead)
// }

// unsafe fn extend_lifetime<'a, T>(r: &'a mut T) -> &'static mut T {
//
//     // one liner
//     // &mut *(r as *mut T)
//
//     // Convert the mutable reference to a raw pointer
//     let raw_ptr: *mut T = r;
//
//     // Convert the raw pointer back to a mutable reference with 'static lifetime
//     &mut *raw_ptr
// }

fn main() -> Result<(), ShahError> {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    // let routes = shah::routes!(models::State, user, phone);

    let mut state = models::State {
        users: user::db::init().expect("user init"),
        phone: phone::db::setup().expect("phone setup"),
        // detail: detail::db::setup().expect("detail setup"),
        // notes: note::db::setup().expect("note setup"),
    }.init()?;

    // let mig = EntityMigration::<
    //     user::db::User,
    //     user::db::User_0,
    //     &'static mut models::State,
    // >::new(user::db::old_init()?, unsafe {
    //     extend_lifetime(&mut state)
    // });
    // state.users.set_migration(mig);
    // let ng = state.users.new_gene();

    let mut _user = user::db::User::default();
    // state.users.add(&mut user)?;

    // log::info!("users: {}", state.users.live);
    //
    // loop {
    //     log::info!("========================");
    //     state.users.work()?;
    //     std::thread::sleep(std::time::Duration::from_secs(2));
    // }

    // println!("tasks: {tasks:?}");

    // for i in 0..200 {
    //     user.gene.id = 0;
    //     user.set_name(&format!("user: {i}"));
    //     state.users.add(&mut user)?;
    // }
    //
    Ok(())

    // match shah::command() {
    //     Commands::Gg => {

    //     }
    //     Commands::Help => {
    //         std::io::stdout().write_all(Commands::help().as_bytes()).unwrap();
    //     }
    //     _ => {}
    // }
    //     Commands::Run => {
    //         shah::server::run(SOCK_PATH, &mut state, &routes).unwrap()
    //     }
    // }

    // Ok(())
}

struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, _md: &log::Metadata) -> bool {
        // metadata.level() <= log::Level::Info
        true
    }

    fn log(&self, record: &log::Record) {
        let level = match record.level() {
            log::Level::Trace => ["\x1b[36m", "T", "Trace"],
            log::Level::Debug => ["\x1b[35m", "D", "Debug"],
            log::Level::Info => ["\x1b[34m", "I", "Info"],
            log::Level::Warn => ["\x1b[33m", "W", "Warn"],
            log::Level::Error => ["\x1b[31m", "E", "Error"],
        };
        println!(
            "[{}{}\x1b[0m]{{{}{}\x1b[32m:\x1b[93m{}\x1b[0m}}: {}",
            level[0],
            level[1],
            level[0],
            record.target(),
            record.line().unwrap_or_default(),
            record.args(),
        );
    }

    fn flush(&self) {}
}
