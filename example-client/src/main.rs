use example_db::user::{
    client::{user_add, user_get},
    db::User,
};
use shah::Taker;

fn main() {
    let mut taker = Taker::init("/tmp/shah.sock", "/tmp/shah.example.sock")
        .expect("could not init taker");

    let mut old_user = User::default();
    old_user.set_phone("12").expect("invalid phone");
    old_user.set_name("my name 😂");
    println!("old user: {old_user:#?}");
    let (new_user,) = user_add(&mut taker, &old_user).unwrap();
    println!("new user: {new_user:#?}");
    let new_user_gene = new_user.gene;

    let (user,) =
        user_get(&mut taker, &new_user_gene).expect("error getting user");
    println!("user name: {:?} - {:?}", user.name, user.name());


    // let name = name.split(|c| *c == 0).next().unwrap();
    // let name = match core::str::from_utf8(name) {
    //     Err(e) => {
    //         match core::str::from_utf8(&name[..e.valid_up_to()]) {
    //             Ok(v) => v,
    //             Err(e) => {
    //                 println!("err: {e}");
    //                 ""
    //             }
    //         }
    //     },
    //     Ok(v) => v
    // };
    // println!("user name: {:?} - {name}", name);
}
