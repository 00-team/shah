use std::{
    os::unix::net::UnixDatagram,
    time::{Duration, Instant},
};

fn main() -> std::io::Result<()> {
    log::set_logger(&SimpleLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Trace);

    let input = std::env::args().nth(1).unwrap_or("client".to_string());
    let path = format!("/tmp/shah.client.{input}.sock");

    let _ = std::fs::remove_file(&path);
    let server = UnixDatagram::bind(&path)?;
    server.connect("/tmp/shah.example.sock")?;
    server.set_read_timeout(Some(Duration::from_secs(5)))?;
    server.set_write_timeout(Some(Duration::from_secs(5)))?;

    log::info!("shah client ...\n");

    let input = input.as_bytes();
    let mut output = [0u8; 4096];

    let mut total_time = (0u128, 0u128);

    loop {
        let time = Instant::now();

        let ss = server.send(input)?;
        let rs = server.recv(&mut output)?;
        assert_eq!(ss, rs);
        assert_eq!(input, &output[..rs]);

        total_time.1 += time.elapsed().as_nanos();
        total_time.0 += 1;

        println!(
            "\x1b[A{} - {}                     ",
            total_time.1 / total_time.0,
            total_time.0
        );
    }
}

struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
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
