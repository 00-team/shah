mod detail;
mod models;
mod note;
mod phone;
mod user;

// use std::io::Write;

use std::io::Write;

use rand::Rng;
use shah::{
    db::trie_const::TrieConstKey, error::ShahError, models::GeneId, Command,
    ShahSchema,
};

const SOCK_PATH: &str = "/tmp/shah.sock";

#[derive(ShahSchema)]
#[shah::model]
struct Whip<const N: usize> {
    // data: [u8; N],
    #[str]
    name: [u8; N],
}

#[derive(Debug, Default, Command)]
enum Commands {
    #[default]
    Help,
    Run,
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

fn random_phone(rng: &mut rand::rngs::ThreadRng) -> TrieConstKey<2> {
    TrieConstKey::<2> {
        cache: rng.gen_range(0..10000000),
        index: [rng.gen_range(0..10), rng.gen_range(0..10)],
    }
}

fn phone_to_str(key: &TrieConstKey<2>) -> String {
    let mut phone = format!("09{:0>7}", key.cache);
    phone.push_str(&key.index[0].to_string());
    phone.push_str(&key.index[1].to_string());
    phone
}

fn main() -> Result<(), ShahError> {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let mut rng = rand::thread_rng();
    let mut phone = phone::db::setup()?;
    let mut users_0 = user::db::init_0()?;
    if users_0.live == 0 {
        let mut user_0 = user::db::User_0::default();
        for i in 0..86 {
            let uphone = loop {
                let key = random_phone(&mut rng);
                if let Err(e) = phone.get(&key) {
                    e.not_found_ok()?;
                    break key;
                }
                continue;
            };
            user_0.set_phone(&phone_to_str(&uphone));
            user_0.gene.id = GeneId(0);
            user_0.set_name(&format!("user_0:{i}"));
            users_0.add(&mut user_0)?;
            phone.set(&uphone, user_0.gene)?;
        }
    }
    drop(phone);

    let routes = shah::routes!(models::State, user, phone, detail);

    log::debug!("init state");
    let mut state = models::State::new(
        user::db::init()?,
        phone::db::setup()?,
        detail::db::setup()?,
        note::db::init()?,
    )?;

    let mut user = user::db::User::default();

    // let gene_57 = Gene { id: 57, iter: 0, pepper: [149, 231, 78], server: 0 };
    // state.users.get(&gene_57, &mut user)?;
    // log::info!("user: {user:#?}");

    // state.users.add(&mut user)?;

    // log::info!("users: {}", state.users.live);
    // let mut npf = 0;
    // let mut dpf = 0;
    // loop {
    //     log::info!("========================");
    //     if !state.users.work()?.0 {
    //         npf += 1;
    //     } else {
    //         dpf += 1;
    //     }
    //     user.gene.id = GeneId(0);
    //     user.set_name("a new user");
    //     state.users.add(&mut user)?;
    //     if dpf > 1 {
    //         break;
    //     }
    //     if npf > 10 {
    //         break;
    //     }
    //     // std::thread::sleep(std::time::Duration::from_secs(2));
    // }

    // Ok(())

    // println!("tasks: {tasks:?}");

    //
    // Ok(())

    match shah::command() {
        Commands::Help => {
            std::io::stdout().write_all(Commands::help().as_bytes())?
        }
        Commands::Run => shah::run(SOCK_PATH, &mut state, &routes)?,
    }

    Ok(())
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
