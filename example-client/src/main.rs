use example_db::{detail, models::ExampleError};
use shah::{ClientError, Taker};

fn act() -> Result<(), ClientError<ExampleError>> {
    let mut taker = Taker::init("/tmp/shah.sock", "/tmp/shah.example.sock")
        .expect("could not init taker");

    for (ch, l) in [('A', 400usize), ('B', 6_000), ('C', 1024)] {
        let len = l.min(detail::DETAIL_MAX);
        let detail = string_data(ch, l);
        let gene = detail::set(&mut taker, &None, &detail)?;
        println!("set: \x1b[32mch: {ch} - {l} - {gene:?}\x1b[m");
        let out = detail::get(&mut taker, &gene)?;
        assert_eq!(detail[..len], out);
        detail::free(&mut taker, &gene)?;
    }

    Ok(())
}

fn main() {
    if let Err(e) = act() {
        println!("error: {e:#?}");
    }
}

fn string_data(ch: char, l: usize) -> String {
    let mut s = String::with_capacity(l);
    for _ in 0..l {
        s.push(ch)
    }
    s
}
