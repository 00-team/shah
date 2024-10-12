use example_db::user::client::user_get;
use shah::{Gene, Taker};

fn main() {
    let mut taker = Taker::init("/tmp/shah.sock", "/tmp/shah.example.sock")
        .expect("could not init taker");

    let user_gene = Gene { id: 1, iter: 0, pepper: [89, 128, 108], ..Default::default() };
    let result = user_get(&mut taker, &user_gene);
    println!("result: {result:#?}");

    // println!("result: {result:?}");
}
