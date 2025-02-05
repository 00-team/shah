// use example_db::{models::ExampleError};
// use shah::{ClientError, Taker};

// fn act() -> Result<(), ClientError<ExampleError>> {
//     let taker = Taker::init("/tmp/shah.sock", "/tmp/shah.example.sock")
//         .expect("could not init taker");
//
//     let _ = taker.connect();
//
//     for (ch, l) in [('A', 400usize), ('B', 6_000), ('C', 1024)] {
//         let len = l.min(detail::DETAIL_MAX);
//         let detail = string_data(ch, l);
//         let gene = detail::set(&taker, &None, &detail)?;
//         println!("set: \x1b[32mch: {ch} - {l} - {gene:?}\x1b[m");
//         let out = detail::get(&taker, &gene)?;
//         assert_eq!(detail[..len], out);
//         if l == 6_000 {
//             detail::free(&taker, &gene)?;
//         }
//     }
//
//     Ok(())
// }

fn main() {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    // if let Err(e) = act() {
    //     println!("error: {e:#?}");
    // }
}

// fn string_data(ch: char, l: usize) -> String {
//     let mut s = String::with_capacity(l);
//     for _ in 0..l {
//         s.push(ch)
//     }
//     s
// }

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
