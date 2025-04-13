use super::{SnakeDb, SnakeHead};
use crate::{
    db::entity::Entity, models::{Binary, Gene}, NotFound, ShahError, SystemError
};
use std::io::{Read, Seek, SeekFrom, Write};

impl SnakeDb {
    pub fn write(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64, data: &[u8],
    ) -> Result<(), ShahError> {
        let len = self.check_offset(gene, head, offset, data.len())?;

        // self.file.seek(SeekFrom::Start(head.position + head.capacity - 1))?;
        // self.file.write_all(&[0u8])?;

        self.file.seek(SeekFrom::Start(head.position + offset))?;
        self.file.write_all(&data[..len])?;

        Ok(())
    }

    pub fn read(
        &mut self, gene: &Gene, head: &mut SnakeHead, offset: u64,
        data: &mut [u8],
    ) -> Result<(), ShahError> {
        let len = self.check_offset(gene, head, offset, data.len())?;
        // log::info!(
        //     "read len: {len} - offset: {offset} - data len: {} - head: {head:#?}",
        //     data.len()
        // );

        self.file.seek(SeekFrom::Start(head.position + offset))?;
        self.file.read_exact(&mut data[..len])?;

        Ok(())
    }

    pub fn set_length(
        &mut self, gene: &Gene, head: &mut SnakeHead, length: u64,
    ) -> Result<(), ShahError> {
        self.index.get(gene, head)?;
        if head.is_free() {
            return Err(NotFound::SnakeIsFree)?;
        }
        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);
        if length > head.capacity {
            log::error!(
                "{} set_length: bad length: {length} >= {}",
                self.ls,
                head.capacity
            );
            return Err(SystemError::SnakeBadLength)?;
        }

        head.length = length;
        self.index.set(head)?;

        Ok(())
    }

    pub fn free(&mut self, gene: &Gene) -> Result<(), ShahError> {
        let mut head = SnakeHead::default();
        self.index.get(gene, &mut head)?;

        assert!(head.position >= SnakeHead::N);
        assert_ne!(head.capacity, 0);

        if head.is_free() {
            log::warn!("freeing already free");
            return Ok(());
        }

        head.set_is_free(true);
        self.index.set(&mut head)?;
        if let Err(e) = self.add_free(head) {
            log::warn!("add_free failed in free: {e:?}");
        }

        Ok(())
    }

    pub fn alloc(
        &mut self, capacity: u64, head: &mut SnakeHead,
    ) -> Result<(), ShahError> {
        if capacity == 0 {
            log::error!("{} alloc: capacity is zero", self.ls);
            return Err(SystemError::SnakeCapacityIsZero)?;
        }

        head.zeroed();
        head.set_alive(true);
        head.set_is_free(false);

        if let Some(free) = self.take_free(capacity)? {
            head.position = free.position;
            head.capacity = free.capacity;
            head.gene = free.gene;
        } else {
            head.position = self.file_size()?;
            if head.position < SnakeHead::N {
                head.position =
                    self.file.seek(SeekFrom::Start(SnakeHead::N))?;
            }
            head.capacity = capacity;
        }

        self.file.seek(SeekFrom::Start(head.position + head.capacity - 1))?;
        self.file.write_all(&[0u8])?;

        if let Err(e) = self.index.set(head) {
            e.not_found_ok()?;
            self.index.add(head)?;
        }

        Ok(())
    }
}
