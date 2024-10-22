mod detail;
mod models;
mod phone;
mod user;

use shah::Command;
use std::{
    default,
    env::Args,
    io::{stdout, Write},
};

const SOCK_PATH: &str = "/tmp/shah.sock";

#[derive(Debug, Default, Command)]
enum Commands {
    #[default]
    Help,
    Run,
    Detail,
}

fn main() {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let routes = [user::api::ROUTES.as_slice(), phone::api::ROUTES.as_slice()];

    let mut state = models::State {
        users: user::db::setup(),
        phone: phone::db::setup(),
        detail: detail::db::setup(),
    };

    match shah::command() {
        Commands::Help => {
            stdout().write_all(Commands::help().as_bytes());
        }
        Commands::Run => {
            shah::server::run(SOCK_PATH, &mut state, &routes).unwrap()
        }
        Commands::Detail => {
            let buf = "this is a simple test for detail".as_bytes();
            println!("buf: {buf:?} - {}", buf.len());
            let head = state.detail.alloc(buf.len() as u64);
            println!("head: {head:#?}");
        }
    }
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
            "[{}{}\x1b[0m]{{{}{}\x1b[0m}}: {}",
            level[0],
            level[1],
            level[0],
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {}
}
