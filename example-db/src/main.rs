mod models;
mod user;

const SOCK_PATH: &str = "/tmp/shah.sock";

fn main() {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let routes = [user::api::ROUTES.as_slice()];

    let mut state = models::State { users: user::db::setup() };

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
