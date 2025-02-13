use super::{EntityItem, EntityKochProg};
use crate::models::{Binary, DbHead, Schema, ShahMagic, ShahMagicDb};
use crate::{DbError, ShahError};

pub const META_OFFSET: u64 = EntityHead::N + EntityKochProg::N;
pub const ENTITY_VERSION: u16 = 1;
pub const ENTITY_MAGIC: ShahMagic =
    ShahMagic::new_const(ShahMagicDb::Entity as u16);

#[crate::model]
pub struct EntityHead {
    pub db_head: DbHead,
    pub item_size: u64,
    pub schema: [u8; 4096],
}

impl EntityHead {
    pub fn check<T: EntityItem>(
        &self, iteration: u16, ls: &str,
    ) -> Result<(), ShahError> {
        if self.db_head.magic != ENTITY_MAGIC {
            log::error!(
                "{ls} head invalid db magic: {:?} != {ENTITY_MAGIC:?}",
                self.db_head.magic
            );
            return Err(DbError::InvalidDbHead)?;
        }

        if self.db_head.db_version != ENTITY_VERSION {
            log::error!(
                "{ls} mismatch db_version {} != {ENTITY_VERSION}",
                self.db_head.db_version,
            );
            return Err(DbError::InvalidDbHead)?;
        }

        if self.db_head.iteration != iteration {
            log::error!(
                "{ls} head invalid iteration {} != {iteration}",
                self.db_head.iteration,
            );
            return Err(DbError::InvalidDbHead)?;
        }

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
                "{ls} mismatch schema. did you forgot to update the iteration?"
            );
            return Err(DbError::InvalidDbSchema)?;
        }

        Ok(())
    }
}
