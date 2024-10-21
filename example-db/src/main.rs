use std::{default, env::Args};

mod models;
mod phone;
mod user;

const SOCK_PATH: &str = "/tmp/shah.sock";

#[shah::command]
#[derive(Debug, Default)]
enum Commands {
    #[default]
    Run,
    DoAction,
    Abc(u8),
    SomeComm {
        id: u16,
        name: String,
    },
}

impl Commands {
    fn parse(mut args: Args) -> Commands {
        let Some(cmd) = args.next() else { return Self::default() };

        match cmd.as_str() {
            "run" => Self::Run,
            "do-action" => Self::DoAction,
            "abc" => {
                let Some(iv0) = args.next() else { return Self::default() };
                Self::Abc(iv0.parse::<u8>().expect("invalid arg for abc != u8"))
            }
            "some-comm" => {
                let mut id = u16::default();
                let mut name = String::default();

                Self::SomeComm { id, name }
            }
            _ => Self::default(),
        }
    }
}

fn main() {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let mut args = std::env::args();
    println!("args: {args:?}");
    let command = loop {
        let Some(arg) = args.next() else { break Commands::default() };
        if arg == "-c" {
            break Commands::parse(args);
        }
    };

    println!("command: {command:#?}");

    return;

    let routes = [user::api::ROUTES.as_slice(), phone::api::ROUTES.as_slice()];

    let mut state =
        models::State { users: user::db::setup(), phone: phone::db::setup() };

    shah::server::run(SOCK_PATH, &mut state, &routes)
        .expect("could not init server");
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
