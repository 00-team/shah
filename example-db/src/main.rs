mod detail;
mod models;
mod note;
mod phone;
mod user;

use crate::models::State;
use rand::Rng;
use shah::{
    db::snake::SnakeHead, error::SystemError, Binary, Command, Gene, BLOCK_SIZE,
};
use std::io::{stdout, Write};

const SOCK_PATH: &str = "/tmp/shah.sock";

#[derive(Debug, Default, Command)]
enum Commands {
    #[default]
    Help,
    Run,
    Detail,
}

pub fn detail_get(
    state: &mut State, gene: &Gene,
) -> Result<(Vec<u8>, SnakeHead), SystemError> {
    let mut head = SnakeHead::default();
    let mut buf = [0u8; BLOCK_SIZE];
    state.detail.read(gene, &mut head, 0, &mut buf)?;

    let len = head.length.min(head.capacity);
    let len = if len == 0 { head.capacity } else { len } as usize;
    // let mut v = Vec::with_capacity(len);
    // unsafe { v.set_len(len) };
    let mut v = vec![0u8; len];

    if len > BLOCK_SIZE {
        v[..BLOCK_SIZE].copy_from_slice(&buf);
        for i in 1..=(len / BLOCK_SIZE) {
            let off = i * BLOCK_SIZE;
            state.detail.read(gene, &mut head, off as u64, &mut buf)?;
            v[off..(off + BLOCK_SIZE).min(len)]
                .copy_from_slice(&buf[..(len - off).min(BLOCK_SIZE)])
        }
    } else {
        v.copy_from_slice(&buf[..len]);
    }

    Ok((v, head))
    // Ok(v.as_utf8_str().to_string())
}

pub fn detail_set(
    state: &mut State, gene: Option<Gene>, data: &[u8],
) -> Result<SnakeHead, SystemError> {
    // assert!(data.len() <= BLOCK_SIZE);

    let len = data.len().min(detail::DETAIL_MAX);
    let mut snake: Option<SnakeHead> = None;
    if let Some(old) = &gene {
        let mut old_head = SnakeHead::default();
        state.detail.index.get(old, &mut old_head)?;
        if old_head.capacity >= len as u64 {
            snake = Some(old_head);
        } else {
            state.detail.free(old)?;
        }
    }
    if snake.is_none() {
        let cap = (len + detail::DETAIL_BUF).min(detail::DETAIL_MAX) as u64;
        let mut head = SnakeHead::default();
        state.detail.alloc(cap, &mut head)?;
        snake = Some(head);
    }
    let snake = snake.unwrap();
    let mut rethead = SnakeHead::default();
    state.detail.write(&snake.gene, &mut rethead, 0, &data[0..len])?;

    for i in 0..=(len / BLOCK_SIZE) {
        let off = i * BLOCK_SIZE;
        if len < (off + BLOCK_SIZE) {
            let mut write_buffer = [0u8; BLOCK_SIZE];
            let wlen = len - off;
            write_buffer[0..wlen].copy_from_slice(&data[off..len]);
            state.detail.write(
                &snake.gene,
                &mut rethead,
                off as u64,
                &write_buffer[0..wlen],
            )?;
        } else {
            state.detail.write(
                &snake.gene,
                &mut rethead,
                off as u64,
                &data[off..off + BLOCK_SIZE],
            )?;
        }
    }

    state.detail.set_length(&snake.gene, &mut rethead, len as u64)?;

    Ok(rethead)
}

fn main() -> Result<(), SystemError> {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let routes = shah::routes!(models::State, user, phone, detail);

    let mut state = models::State {
        users: user::db::setup().expect("user setup"),
        phone: phone::db::setup().expect("phone setup"),
        detail: detail::db::setup().expect("detail setup"),
        notes: note::db::setup().expect("note setup"),
    };

    match shah::command() {
        Commands::Help => {
            stdout().write_all(Commands::help().as_bytes()).unwrap();
        }
        Commands::Run => {
            shah::server::run(SOCK_PATH, &mut state, &routes).unwrap()
        }
        Commands::Detail => {
            let mut rng = rand::thread_rng();
            let mut pool = Vec::<(u8, u64, Gene)>::with_capacity(10_000);
            let mut set_buf = [0u8; 10 * 1024];
            let mut get_buf = [0u8; 10 * 1024];

            let mut total_length = 0u64;
            let mut total_capacity = 0u64;

            for _ in 0usize..10_000 {
                let len = rng.gen_range(1u64..5049) + 8;
                let ulen = len as usize;
                let char = rng.gen_range(b'a'..b'z');
                set_buf[0..ulen - 8].fill(char);
                set_buf[ulen - 8..ulen].clone_from_slice(&len.to_le_bytes());
                let head = detail_set(&mut state, None, &set_buf[0..ulen])?;
                assert_eq!(head.length, len);
                pool.push((char, len, head.gene));
                total_length += len;
                total_capacity += head.capacity;
                assert_eq!(head.length, len, "set length is not correct");
            }

            println!("total_length: {total_length}");
            println!("total_capacity: {total_capacity}");
            println!(
                "total_capacity + SnakeHead::N: {}",
                total_capacity + SnakeHead::N
            );

            for (char, len, gene) in pool.iter() {
                let ulen = *len as usize;
                let (data, head) = detail_get(&mut state, gene)?;

                assert_eq!(data.len(), ulen, "invalid data len");
                assert_eq!(*len, head.length, "invalid head len");
                assert!(
                    head.length <= head.capacity,
                    "invalid length <= capacity"
                );
                get_buf[0..ulen - 8].fill(*char);
                assert_eq!(data[..ulen - 8], get_buf[0..ulen - 8], "bad data");
                let data_len = u64::from_le_bytes(
                    data[ulen - 8..ulen].try_into().unwrap(),
                );
                assert_eq!(data_len, *len, "invalid len in data");
            }

            log::info!("deleting all details");
            // pool.shuffle(&mut rng);
            for (i, (_, _, gene)) in pool.iter().enumerate() {
                if i % 2 == 0 {
                    continue;
                }
                state.detail.free(gene)?;
            }

            for (i, (_, _, gene)) in pool.iter().enumerate() {
                if i % 2 != 0 {
                    continue;
                }
                state.detail.free(gene)?;
            }

            // log::info!("reset all details");
            // let mut new_total_length = 0u64;
            // let mut new_total_capacity = 0u64;
            //
            // for (char, len, gene) in pool.iter_mut() {
            //     *len = rng.gen_range(4000u64..9549) + 8;
            //     let ulen = *len as usize;
            //     *char = rng.gen_range(b'a'..b'z');
            //     set_buf[0..ulen - 8].fill(*char);
            //     set_buf[ulen - 8..ulen].clone_from_slice(&len.to_le_bytes());
            //     let head =
            //         detail_set(&mut state, Some(*gene), &set_buf[0..ulen])?;
            //     *gene = head.gene;
            //     assert_eq!(head.length, *len, "invalid head length");
            //     // pool.push((*char, len, head.gene));
            //     new_total_length += *len;
            //     new_total_capacity += head.capacity;
            // }
            //
            // println!("new_total_length: {new_total_length}");
            // println!("new_total_capacity: {new_total_capacity}");
            //
            // for (char, len, gene) in pool.iter() {
            //     let ulen = *len as usize;
            //     let (data, head) = detail_get(&mut state, gene)?;
            //
            //     assert_eq!(data.len(), ulen, "invalid data len");
            //     assert_eq!(*len, head.length, "invalid head len");
            //     assert!(
            //         head.length <= head.capacity,
            //         "invalid length <= capacity"
            //     );
            //     get_buf[0..ulen - 8].fill(*char);
            //     assert_eq!(data[..ulen - 8], get_buf[0..ulen - 8], "bad data");
            //     let data_len = u64::from_le_bytes(
            //         data[ulen - 8..ulen].try_into().unwrap(),
            //     );
            //     assert_eq!(data_len, *len, "invalid len in data");
            // }
        }
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
