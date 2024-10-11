
struct ResponseHead {
    status: u32,
    size: u32,
    elapsed: f64,
}

struct Gene {
    id: u32,
    pepper: u32,
    server: u32,
    version: u8,
    idx: u8,
    iter: u8,
    _reserved: u8,
}

struct Detail {
    flag: u64,
    gene: Gene,
    size: u64,
    position: u64,
    length: u64,
}

struct Record {
    flag: u64,
    gene: Gene,
    checksum: [u8; 16],
    server: u32,
    width: u32,
    height: u32,
    size: u32,
    ext: u8,
    _reserved: [u8; 3],
    duration: f32,
}

struct Agent {
    flag: u64,
    gene: Gene,
    user: Gene,
    admin_perms: [u8; 64],
}

struct Duration {
    open: u8,
    close: u8,
}

struct Eatery {
    flag: u64,
    gene: Gene,
    latitude: f64,
    longitude: f64,
    menu: Gene,
    review: Gene,
    detail: Gene,
    extra: Gene,
    photos: [Gene; 7],
    stars: [u64; 5],
    theme: u32,
    cc: u16,
    tables: i16,
    category: u8,
    zoom: u8,
    phone: [u8; 12], // string
    opening_hours: [[Duration; 4]; 7],
    name: [u8; 58], // string
}

struct Dish {
    flag: u64,
    gene: Gene,
    ty: u8,
    name: [u8; 128], // str
    note: [u8; 127], // str
    photos: [Gene; 4],
    price: i64,
}

struct Review {
    flag: u64,
    gene: Gene,
    target: Gene,
    cousin: Gene,
    detail: Gene,
    timestamp: u64,
    star: u8,
    state: u8,
    summary: [u8; 222], // str
}

struct BlockHeader {
    flag: u64,
    gene: Gene,
    index: Gene,
    past: Gene,
    next: Gene,
    live: u8,
    _reserved: [u8; 7],
}

struct ReviewBlock {
    header: BlockHeader,
    reviews: [Review; 32],
}

struct PondIndex {
    flag: u64,
    gene: Gene,
    owner: Gene,
    block_count: u64,
    item_count: u64,
    first: Gene,
    last: Gene,
}

struct MenuBlock {
    header: BlockHeader,
    menu: [Dish; 32],
}

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

struct Session {
    ip: [u8; 4],
    info: SessionInfo,
    timestamp: u64,
    token: [u8; 64],
}

struct User {
    flag: u64,
    gene: Gene,
    agent: Gene,
    review: Gene,
    photo: Gene,
    reviews: [u64; 3],
    phone: [u8; 12], // str
    cc: u16,
    name: [u8; 50],
    sessions: [Session; 3],
}

// concept
// modelxx! {
//     /// Hi
//     #[derive(Debug, PartialEq, Clone, Copy)]
//     User {
//         flags: u64,
//         gene: Gene,
//         agent: Gene,
//         review: Gene,
//         photo: Gene,
//         reviews[3]: u64,
//         #[str(abc="0123456789")]
//         phone[12]: u8,
//         cc: u16,
//         #[str]
//         name[50]: u8,
//         sessions[3]: Session {
//             ip[4]: u8,
//             info: SessionInfo {
//                 client: u8,
//                 os: u8,
//                 browser: u8,
//                 device: u8,
//                 client_version: u16,
//                 os_version: u16,
//                 browser_version: u16,
//                 _pad: u16,
//             },
//             timestamp: u64,
//             token[64]: u8,
//         }
//     }
// }
