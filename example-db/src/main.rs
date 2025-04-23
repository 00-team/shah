mod detail;
mod extra;
mod map;
mod models;
mod note;
mod phone;
mod user;
mod username;

use example_db::models::ExampleError;
use rand::Rng;
use shah::{Command, db::trie_const::TrieConstKey, error::ShahError};

const SOCK_PATH: &str = "/tmp/shah.example-db.sock";

#[derive(Debug, Default, Command)]
enum Commands {
    #[default]
    Help,
    Run,
}

#[allow(dead_code)]
fn random_phone(rng: &mut rand::rngs::ThreadRng) -> TrieConstKey<2> {
    TrieConstKey::<2> {
        cache: rng.gen_range(0..10000000),
        index: [rng.gen_range(0..10), rng.gen_range(0..10)],
    }
}

#[allow(dead_code)]
fn phone_to_str(key: &TrieConstKey<2>) -> String {
    let mut phone = format!("09{:0>7}", key.cache);
    phone.push_str(&key.index[0].to_string());
    phone.push_str(&key.index[1].to_string());
    phone
}

fn main() -> Result<(), ShahError> {
    unsafe { std::env::set_var("SHAH_SERVER_INDEX", "7") };
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    // let mut map = map::db::MapDb::new()?;

    // let mut rng = rand::thread_rng();
    // let mut phone = phone::db::setup()?;
    // let mut users_0 = user::db::init_0()?;
    // if users_0.live == 0 {
    //     let mut user_0 = user::db::User_0::default();
    //     for i in 0..86 {
    //         let uphone = loop {
    //             let key = random_phone(&mut rng);
    //             if let Err(e) = phone.get(&key) {
    //                 e.not_found_ok()?;
    //                 break key;
    //             }
    //             continue;
    //         };
    //         user_0.set_phone(&phone_to_str(&uphone));
    //         user_0.gene.id = GeneId(0);
    //         user_0.set_name(&format!("user_0:{i}"));
    //         users_0.add(&mut user_0)?;
    //         phone.set(&uphone, user_0.gene)?;
    //     }
    // }
    // drop(phone);

    let routes = shah::routes!(models::State, user, phone, detail);

    log::debug!("init state");
    let mut state = models::State::new(
        user::db::init()?,
        phone::db::setup()?,
        detail::db::setup()?,
        note::db::init()?,
        extra::db::init()?,
    )?;

    // let gene_57 = Gene { id: 57, iter: 0, pepper: [149, 231, 78], server: 0 };
    // state.users.get(&gene_57, &mut user)?;
    // log::info!("user: {user:#?}");

    // state.users.add(&mut user)?;

    // log::info!("users: {}", state.users.live);
    // let mut npf = 0;
    // let mut dpf = 0;

    // let exit = Arc::new(AtomicBool::new(false));
    // shah::signals::register_exit(&exit)?;
    // loop {
    //     log::debug!("doing something");
    //     if exit.load(Ordering::Relaxed) {
    //         log::info!("exiting");
    //         break;
    //     }
    // if !state.users.work()?.0 {
    //     npf += 1;
    // } else {
    //     dpf += 1;
    // }
    // user.gene.id = GeneId(0);
    // user.set_name("a new user");
    // state.users.add(&mut user)?;
    // if dpf > 1 {
    //     break;
    // }
    // if npf > 10 {
    //     break;
    // }
    //     std::thread::sleep(std::time::Duration::from_millis(700));
    // }

    log::info!("commands");
    match shah::command() {
        Commands::Help => {
            println!("{}", Commands::help())
        }
        Commands::Run => {
            shah::Server::new(
                SOCK_PATH,
                &mut state,
                &routes,
                ExampleError::Unknown,
            )?
            .run()?;
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
