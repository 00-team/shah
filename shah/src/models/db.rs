use crate::SHAH_VERSION;

#[crate::model]
#[derive(Debug, PartialEq, Eq)]
pub struct ShahMagic {
    sign: [u8; 5],
    prefix: u8,
    db: u16,
}

#[crate::enum_int(ty = u16)]
#[derive(Debug, Default)]
pub enum ShahMagicDb {
    #[default]
    Unknown,
    Entity,
    Pond,
    Snake,
    TrieConst,
}

impl ShahMagic {
    const SIGN: [u8; 5] = *b"\x07SHAH";
    const PREFIX: u8 = 7;

    pub fn new(db: ShahMagicDb) -> Self {
        Self { sign: Self::SIGN, prefix: Self::PREFIX, db: db.into() }
    }

    pub const fn new_const(db: u16) -> Self {
        Self { sign: Self::SIGN, prefix: Self::PREFIX, db }
    }

    pub fn custom<Db: Into<u16>>(prefix: u8, db: Db) -> Self {
        assert_ne!(
            prefix,
            Self::PREFIX,
            "for custom databases you cannot use the shah prefix"
        );
        Self { sign: Self::SIGN, prefix, db: db.into() }
    }

    pub fn is_valid(&self) -> bool {
        self.sign == Self::SIGN
    }

    pub fn is_custom(&self) -> bool {
        self.prefix != Self::PREFIX
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    pub fn raw_db(&self) -> u16 {
        self.db
    }

    pub fn db(&self) -> ShahMagicDb {
        ShahMagicDb::from(self.db)
    }
}

#[crate::model]
#[derive(Debug)]
pub struct DbHead {
    pub magic: ShahMagic,
    pub shah_version: (u16, u16),
    pub db_version: u16,
    pub iteration: u16,
    #[str]
    pub name: [u8; 48],
}

impl DbHead {
    pub fn new(
        magic: ShahMagic, iteration: u16, name: &str, version: u16,
    ) -> Self {
        let mut head = Self::default();
        head.init(magic, iteration, name, version);
        head
    }

    pub fn init(
        &mut self, magic: ShahMagic, iteration: u16, name: &str, version: u16,
    ) {
        self.magic = magic;
        self.iteration = iteration;
        self.shah_version = SHAH_VERSION;
        self.db_version = version;
        self.set_name(name);
    }
}
