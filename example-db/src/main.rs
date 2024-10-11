mod models;
mod post;
mod user;

const SOCK_PATH: &str = "/tmp/shah.example.sock";

fn main() {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let routes = [user::ROUTES.as_slice(), post::ROUTES.as_slice()];

    let mut state =
        models::State { users: user::setup(), posts: post::setup() };

    shah::server::run(SOCK_PATH, &mut state, &routes)
        .expect("could not init server");

    // let mut order = [0u8; 1024 * 64];
    // let mut reply = [0u8; 1024 * 64];
    //
    // order.iter_mut().enumerate().for_each(|(i, x)| *x = i as u8);
    //

    // log::debug!("routes: {routes:#?}");
    // let route = &routes[0][0];
    // let res = (route.caller)(
    //     &state,
    //     &order[0..route.input_size],
    //     &mut reply[0..route.output_size],
    // );
    // log::debug!("route: {route:#?}");
    // // let res = (routes[0][0].caller)(&state, &order, &mut reply);
    // log::debug!("res: {res:?}")

    // api::router(0, &state, &order, &mut reply);

    // server::init(db).expect("server init error");

    // let mut main_buffer = [0u8; 4096];
    // let user_slice = &mut main_buffer[0..User::SIZE];
    // let (head, user, rest) = unsafe { user_slice.align_to_mut::<User>() };
    // assert!(head.is_empty() && rest.is_empty(), "data did not align");
    // let user = &mut user[0];
    //
    // let mut db = user::setup();
    // db.update_population();
    // // db.add(user);
    // log::debug!("user size: {}", User::S);
    // log::debug!("db: live: {} | dead: {}", db.live, db.dead);
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
            "[{}{}\x1b[0m]{{\x1b[32m{}\x1b[0m}}: {}",
            level[0],
            level[1],
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {}
}
