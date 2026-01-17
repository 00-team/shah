use super::{FREE_LIST_SIZE, SnakeDb, SnakeFree, TCD};
use crate::{
    ShahError,
    db::{entity::Entity, snake::SnakeHead},
    models::Gene,
};

impl SnakeDb {
    pub(super) fn take_free(
        &mut self, capacity: u64,
    ) -> Result<Option<SnakeFree>, ShahError> {
        let db_size = self.file_size()?;

        let mut travel = 0;
        for opt_free in self.free_list.iter_mut() {
            if travel >= self.free {
                break;
            }
            let Some(free) = opt_free else { continue };
            travel += 1;

            assert_ne!(free.position, 0);
            assert_ne!(free.capacity, 0);
            assert_ne!(free.gene.id, 0);

            if free.position + free.capacity == db_size {
                if free.capacity > capacity + TCD {
                    let val = SnakeFree {
                        position: free.position,
                        capacity,
                        gene: Gene::default(),
                    };

                    let mut disk = SnakeHead::default();
                    self.index.get(&free.gene, &mut disk)?;
                    if !disk.is_free() {
                        log::warn!("free is not even free on the disk :/ wtf");
                        return Ok(None);
                    }
                    if disk.capacity != free.capacity
                        || disk.position != free.position
                    {
                        log::warn!(
                            "invalid snake free_list: {disk:?} != {free:?}"
                        );
                        return Ok(None);
                    }

                    free.position += capacity;
                    free.capacity -= capacity;

                    disk.position = free.position;
                    disk.capacity = free.capacity;
                    self.index.set(&mut disk)?;

                    return Ok(Some(val));
                }

                let val = SnakeFree {
                    position: free.position,
                    capacity: free.capacity.max(capacity),
                    gene: free.gene,
                };

                if self.free > 0 {
                    self.free -= 1;
                }
                *opt_free = None;
                return Ok(Some(val));
            }

            if free.capacity < capacity {
                continue;
            }

            if free.capacity - capacity < TCD {
                if self.free > 0 {
                    self.free -= 1;
                }
                let val = *opt_free;
                *opt_free = None;
                return Ok(val);
            }

            let mut disk = SnakeHead::default();
            self.index.get(&free.gene, &mut disk)?;
            if !disk.is_free() {
                log::warn!("free is not even free :/ wtf");
                return Ok(None);
            }
            if disk.capacity != free.capacity || disk.position != free.position
            {
                log::warn!("invalid snake free_list: {disk:?} != {free:?}");
                return Ok(None);
            }

            disk.capacity -= capacity;
            self.index.set(&mut disk)?;
            free.capacity -= capacity;
            return Ok(Some(SnakeFree {
                gene: Default::default(),
                position: free.position + free.capacity,
                capacity,
            }));
        }

        Ok(None)
    }

    pub(super) fn add_free(
        &mut self, mut head: SnakeHead,
    ) -> Result<(), ShahError> {
        if head.position == 0 || head.capacity == 0 || head.gene.id == 0 {
            log::warn!("invalid head into add_free");
            return Ok(());
        }

        let mut index = if self.free == 0 { 0 } else { FREE_LIST_SIZE };
        let mut travel = 0;
        // let mut round_two = false;
        // let mut round_two_index = 0usize;
        // log::warn!("add_free: {} | {}", head.gene.id, self.free);

        let mut fdx = 0usize;
        while fdx < FREE_LIST_SIZE {
            if travel > self.free {
                break;
            }
            let slot = &mut self.free_list[fdx];
            let Some(free) = slot else {
                if index == FREE_LIST_SIZE {
                    index = fdx;
                }
                fdx += 1;
                continue;
            };
            travel += 1;

            assert_ne!(free.position, 0);
            assert_ne!(free.capacity, 0);
            assert_ne!(free.gene.id, 0);

            // log::trace!(
            //     "[{}] head: {} + {} == {}",
            //     head.gene.id,
            //     head.position,
            //     head.capacity,
            //     head.position + head.capacity
            // );
            // log::trace!(
            //     "[{}] free: {} + {} == {}",
            //     free.gene.id,
            //     free.position,
            //     free.capacity,
            //     free.position + free.capacity
            // );
            // log::trace!("round_two: {round_two}");

            if free.position + free.capacity == head.position {
                // log::info!("found before: round two: {round_two}");
                let mut disk = SnakeHead::default();
                self.index.get(&free.gene, &mut disk)?;

                if disk.capacity != free.capacity
                    || disk.position != free.position
                {
                    log::warn!("disk != free: {disk:?} - {free:?}");
                    return Ok(());
                }

                head.position = free.position;
                head.capacity += free.capacity;

                self.index.del(&disk.gene, &mut SnakeHead::default())?;
                *slot = None;
                if self.free > 0 {
                    self.free -= 1;
                }

                // round_two = true;
                fdx = 0;
                travel = 0;
                continue;

                // old.capacity += head.capacity;
                // free.capacity += head.capacity;
                // if let Err(e) = self.index.set(&old) {
                //     log::warn!("set old free: {e:?} - {:?}", free.gene);
                //     return;
                // }
                // if let Err(e) =
                //     self.index.del(&head.gene, &mut SnakeHead::default())
                // {
                //     log::warn!("del head: {e:?} - {:?}", head.gene);
                //     return;
                // }
                //
                // log::info!("go for round two");
                // round_two = true;
                // fdx = 0;
                // head.position = free.position;
                // head.capacity = free.capacity;
                // head.gene = free.gene;
                // travel = 0;
                // continue;
            }

            if head.position + head.capacity == free.position {
                // log::info!("found after : round two: {round_two}");
                let mut disk = SnakeHead::default();
                self.index.get(&free.gene, &mut disk)?;

                if disk.capacity != free.capacity
                    || disk.position != free.position
                {
                    log::warn!("disk != free: {disk:?} - {free:?}");
                    return Ok(());
                }

                head.capacity += free.capacity;

                self.index.del(&disk.gene, &mut SnakeHead::default())?;
                *slot = None;
                if self.free > 0 {
                    self.free -= 1;
                }

                // free.position = head.position;
                // free.capacity = head.capacity;
                // free.gene = head.gene;
                // self.index.set(&head)?;
                // head.position = free.position;
                // head.capacity = free.capacity;

                // round_two = true;
                fdx = 0;
                travel = 0;
                continue;
            }

            fdx += 1;
        }

        head.set_is_free(true);
        head.set_alive(true);
        self.index.set(&mut head)?;

        // log::error!("setting a free at: {index} | round_two: {round_two}");
        if index < FREE_LIST_SIZE {
            let opt_free = &mut self.free_list[index];
            if opt_free.is_some() {
                log::warn!("invalid free index. item space occupied: {index}");
                return Ok(());
            }
            *opt_free = Some(SnakeFree {
                position: head.position,
                capacity: head.capacity,
                gene: head.gene,
            });
            self.free += 1;
        }

        Ok(())
    }
}
