// use example_db::user::{
//     client::{user_add, user_get},
//     db::User,
// };
// use shah::Taker;

fn main() {
    // let mut taker = Taker::init("/tmp/shah.sock", "/tmp/shah.example.sock")
    //     .expect("could not init taker");
    //
    // let old_user = User {
    //     age: 69,
    //     name: [48, 48, 55, 32, 239, 159, 152, 130, 0, 0, 0, 0],
    //     ..Default::default()
    // };
    // println!("old user: {old_user:#?}");
    // let (new_user,) = user_add(&mut taker, &old_user).unwrap();
    // println!("new user: {new_user:#?}");
    // let new_user_gene = new_user.gene;
    //
    // let (user,) =
    //     user_get(&mut taker, &new_user_gene).expect("error getting user");
    // println!("user name: {:?}", user.name);

    let name = &[48, 48, 55, 32, 26, 159, 152, 129, 0, 0, 0, 0];

    let name = name.split(|c| *c == 0).next().unwrap();
    let name = match core::str::from_utf8(name) {
        Err(e) => {
            match core::str::from_utf8(&name[..e.valid_up_to()]) {
                Ok(v) => v,
                Err(e) => {
                    println!("err: {e}");
                    ""
                }
            }
        },
        Ok(v) => v
    };
    println!("user name: {:?} - {name}", name);
}
