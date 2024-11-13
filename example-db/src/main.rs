mod detail;
mod models;
mod note;
mod phone;
mod user;

use crate::note::db::Note;
use rand::seq::SliceRandom;
use shah::{db::pond::Origin, error::SystemError, Command};
use std::io::{stdout, Write};

const SOCK_PATH: &str = "/tmp/shah.sock";

#[derive(Debug, Default, Command)]
enum Commands {
    #[default]
    Help,
    Run,
    Note,
}

fn main() -> Result<(), SystemError> {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let routes = [
        user::api::ROUTES.as_slice(),
        phone::api::ROUTES.as_slice(),
        detail::api::ROUTES.as_slice(),
    ];

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
        Commands::Note => {
            let mut origin_pool = Vec::<Origin>::new();
            for _ in 0..5 {
                let mut origin = Origin::default();
                state.notes.origins.add(&mut origin)?;
                origin_pool.push(origin);
            }
            for i in 0..500 {
                let mut note = Note::default();
                note.set_note(&format!(
                    "note: {i} - {}",
                    rand::random::<u16>()
                ));
                let og = origin_pool.choose(&mut rand::thread_rng()).unwrap();
                state.notes.add(&og.gene, &mut note)?;
            }
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
