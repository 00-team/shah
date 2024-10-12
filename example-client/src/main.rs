use shah::{Binary, Gene, Taker};

fn main() {
    // let taker = Taker::init("/tmp/shah.sock", "/tmp/shah.example.sock");
    //
    let user_gene = Gene { id: 12, iter: 19, pepper: [12, 99, 7], ..Default::default() };
    println!("gene: {user_gene:?}");

    // let result = example_db::user::client::user_get(&mut taker, &user_gene);

    let mut buf = [0u8; 30];
    buf[..Gene::S].clone_from_slice(user_gene.as_binary());

    println!("buf: {buf:?}");

    // println!("result: {result:?}");
}
