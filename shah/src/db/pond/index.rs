use super::{Origin, Pond, PondDb, PondItem};
use crate::db::entity::{EntityItem, EntityKochFrom};
use crate::models::Gene;
use crate::PAGE_SIZE;
use crate::{IsNotFound, ShahError};
use std::ops::AddAssign;

impl<T: PondItem + EntityKochFrom<O, S>, O: EntityItem, S> PondDb<T, O, S> {
    pub fn pond_list(
        &mut self, pond: &mut Pond, result: &mut [T; PAGE_SIZE],
    ) -> Result<(), ShahError> {
        let pond_gene = pond.gene;
        self.index.get(&pond_gene, pond)?;
        self.items.list(pond.stack, result)?;
        Ok(())
    }

    pub fn pond_free(&mut self, pond: &mut Pond) -> Result<(), ShahError> {
        let mut buf = [T::default(); PAGE_SIZE];
        self.items.list(pond.stack, &mut buf)?;

        pond.empty = 0;
        for item in buf.iter_mut() {
            if item.is_alive() {
                item.growth_mut().add_assign(1);
                item.set_alive(false);
            }
            if !item.gene().exhausted() {
                pond.empty += 1;
            }
        }

        self.items.write_buf_at(&buf, pond.stack)?;

        pond.set_is_free(true);
        pond.alive = 0;

        self.index.set(pond)?;
        self.free_list.push(pond.gene);

        Ok(())
    }

    pub fn cascade(&mut self, origene: &Gene) -> Result<(), ShahError> {
        let mut origin = Origin::default();
        self.origins.get(origene, &mut origin)?;

        let mut pond_gene = origin.head;
        let mut pond = Pond::default();
        loop {
            if let Err(e) = self.index.get(&pond_gene, &mut pond) {
                if e.is_not_found() {
                    break;
                }
                return Err(e)?;
            }
            pond_gene = pond.next;
            self.pond_free(&mut pond)?;
        }

        self.origins.del(origene, &mut origin)?;

        Ok(())
    }
}
