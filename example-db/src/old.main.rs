mod models;

use std::{
    arch::asm,
    array::TryFromSliceError,
    fmt::Debug,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    mem::{offset_of, size_of},
    slice::from_raw_parts_mut,
};

use plutus::{
    entity::{Entity, EntityDb, EntityHead},
    error::PlutusError,
    Gene,
};
use plutus_macros::model;

// #[allow(dead_code)]
// struct Database {
//     name: &'static str,
//     file: std::fs::File,
// }
//
// #[allow(dead_code)]
// fn user_db() -> &'static Database {
//     static STATE: OnceLock<Database> = OnceLock::new();
//     STATE.get_or_init(|| Database {
//         name: "user",
//         file: std::fs::OpenOptions::new()
//             .read(true)
//             .write(true)
//             .create(true)
//             .open("data/user.bin")
//             .expect("could not open data/user.bin"),
//     })
// }

fn user_get(_a: Gene, _b: [u8; 16]) -> User {
    User::default()
}

// fn service_factory<'a, F, Args>(api: F)
// where
//     F: Handler<Args>,
//     Args: Debug + FromBytes,
//     F::Output: Debug + IntoBytes<'a>,
// {
//     let a = Gene {
//         id: 78,
//         server: 99,
//         ..Default::default()
//     };
//     let b = Gene {
//         id: 11,
//         pepper: 666,
//         ..Default::default()
//     };
//     let mut data: [u8; 32] = [0; 32];
//     data[0..16].clone_from_slice(a.into_bytes());
//     data[16..32].clone_from_slice(b.into_bytes());
//     let args = Args::from_bytes(&data).expect("invalid args");
//     println!("args: {args:#?}");
//     // let response = api.call(args);
//     // println!("response: {response:#?}");
// }

#[model]
#[derive(Debug, PartialEq, Clone, Copy)]
struct SessionInfo {
    client: u8,
    os: u8,
    browser: u8,
    device: u8,
    client_version: u16,
    os_version: u16,
    browser_version: u16,
    _reserved: u16,
}

#[model]
#[derive(Debug, PartialEq, Clone, Copy)]
struct Session {
    ip: [u8; 4],
    info: SessionInfo,
    timestamp: u64,
    token: [u8; 64],
}

#[model]
/// Hi
#[derive(Debug, PartialEq, Clone, Copy)]
struct User {
    flags: u64,
    gene: Gene,
    agent: Gene,
    review: Gene,
    photo: Gene,
    reviews: [u64; 3],
    phone: [u8; 12],
    cc: u16,
    name: [u8; 50],
    sessions: [Session; 3],
}

impl Entity for User {
    fn head(&mut self) -> EntityHead {
        let flags = unsafe { from_raw_parts_mut(&mut self.flags as *mut u64 as *mut u8, 8) };
        EntityHead {
            flags: &mut flags[0],
            gene: &mut self.gene,
        }
    }
}

fn main() {
    let db = EntityDb::new("user", User::default()).expect("could not make a new db");

    // let mut db = UserDb::init();
    // db.update_population().expect("update pop");
    // println!("pop: live: {} | dead: {}", db.live, db.dead);

    // let mut user = User::default();
    // user.cc = 99;
    // db.add(user);

    // println!("user: {user:?}");
    // println!("pop: live: {} | dead: {}", db.pop.live, db.pop.dead);

    // let pop = db.uppopulation().expect("pop err");
    // println!("pop: live: {} | dead: {}", pop.live, pop.dead);

    // service_factory(user_get);
    //
    // let mut gene = Gene {
    //     id: 98,
    //     ..Default::default()
    // };
    // let data = gene.into_bytes();
    // println!("gene: {gene:?}\n{:?}", data);
    // gene.id = 11;
    // println!("gene: {gene:?}\n{:?}", data);

    // server.add_route(User::get).add_route(User::add).add_route(User::set);
}
