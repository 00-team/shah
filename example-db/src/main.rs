mod detail;
mod models;
mod note;
mod phone;
mod user;

use crate::note::db::Note;
use crate::phone::db::{PhoneAbc, PhoneDb};
use rand::seq::SliceRandom;
use shah::{db::pond::Origin, error::SystemError, Command, Gene};
use std::io::{stdout, Write};

const SOCK_PATH: &str = "/tmp/shah.sock";

#[derive(Debug, Default, Command)]
enum Commands {
    #[default]
    Help,
    Run,
    Note,
    Trie,
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
            let mut note_pool = Vec::<Gene>::with_capacity(500);
            for i in 0..500 {
                let mut note = Note::default();
                note.set_note(&format!(
                    "note: {i} - {} - {}",
                    ["\n\nl2", "\n\n\nl3", "\nl1", "xxx\nxxx\nxxx\n"]
                        .choose(&mut rand::thread_rng())
                        .unwrap(),
                    rand::random::<u16>()
                ));
                let og = origin_pool.choose(&mut rand::thread_rng()).unwrap();
                state.notes.add(&og.gene, &mut note)?;
                note_pool.push(note.gene);
            }
            note_pool.shuffle(&mut rand::thread_rng());
            for _ in 0..499 {
                let mut note = Note::default();
                let ng = note_pool.pop().unwrap();
                state.notes.get(&ng, &mut note)?;
                note.set_note("deleted");
                state.notes.set(&mut note)?;
                state.notes.del(&ng, &mut note)?;
            }
            for i in 0..500 {
                let mut note = Note::default();
                note.set_note(&format!(
                    "note: {i} - {} - {}",
                    ["\n\nl2", "\n\n\nl3", "\nl1", "xxx\nxxx\nxxx\n"]
                        .choose(&mut rand::thread_rng())
                        .unwrap(),
                    rand::random::<u16>()
                ));
                let og = origin_pool.choose(&mut rand::thread_rng()).unwrap();
                state.notes.add(&og.gene, &mut note)?;
                note_pool.push(note.gene);
            }
        }
        Commands::Trie => {
            let db = PhoneDb::new("tests.phone", PhoneAbc);
            db.file.set_len(0).expect("file truncate");
            let mut db = db.setup().expect("phone setup");

            let mock_data = [
                // ("223334044", 2233340, [4, 4]),
                // ("183937071", 1839370, [7, 1]),
                // ("192236504", 1922365, [0, 4]),
                ("961772969", 9617729, [6, 9]),
                ("961772970", 9617729, [7, 0]),
            ];

            for (i, (phone, cache, index)) in mock_data.iter().enumerate() {
                let i = i as u64;
                let a = Gene { id: i + 3, ..Default::default() };
                let b = Gene { id: (i + 3) * 2, ..Default::default() };
                let k = db.convert_key(phone).expect("convert key");
                assert_eq!(k.cache, *cache);
                assert_eq!(k.index, *index);

                assert_eq!(db.get(&k).expect("get"), None);
                assert_eq!(db.set(&k, a).expect("set"), None);
                assert_eq!(db.get(&k).expect("get"), Some(a));
                assert_eq!(db.set(&k, b).expect("set"), Some(a));
                assert_eq!(db.get(&k).expect("get"), Some(b));
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
