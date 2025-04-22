use super::{EntityItem, EntityKochProg};
use crate::models::{Binary, DbHead, Schema, ShahMagic, ShahMagicDb};
use crate::{DbError, ShahError};

pub const ENTITY_META: u64 = EntityHead::N + EntityKochProg::N;
pub const ENTITY_VERSION: u16 = 1;
pub const ENTITY_MAGIC: ShahMagic =
    ShahMagic::new_const(ShahMagicDb::Entity as u16);

#[crate::model]
#[derive(Debug)]
pub struct EntityHead {
    pub db_head: DbHead,
    pub item_size: u64,
    pub schema: [u8; 4096],
}

impl EntityHead {
    pub fn check<T: EntityItem>(
        &self, revision: u16, ls: &str,
    ) -> Result<(), ShahError> {
        self.db_head.check(ls, ENTITY_MAGIC, revision, ENTITY_VERSION)?;

        if self.item_size != T::N {
            log::error!(
                "{ls} schema.item_size != current item size. {} != {}",
                self.item_size,
                T::N
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        let schema = Schema::decode(&self.schema)?;
        if schema != T::shah_schema() {
            log::error!(
                "{ls} mismatch schema. did you forgot to update the revision?"
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        Ok(())
    }
}
