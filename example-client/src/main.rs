use example_db::user::{client::{user_add, user_get}, db::User};
use shah::Taker;

fn main() {
    let mut taker = Taker::init("/tmp/shah.sock", "/tmp/shah.example.sock")
        .expect("could not init taker");

    let old_user = User { age: 69, ..Default::default() };
    println!("old user: {old_user:#?}");
    let (new_user, ) = user_add(&mut taker, &old_user).unwrap();
    println!("new user: {new_user:#?}");
    let new_user_gene = new_user.gene;

    let res = user_get(&mut taker, &new_user_gene);
    println!("res: {res:#?}");
}
