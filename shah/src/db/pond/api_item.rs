use super::*;
use crate::ShahError;
use crate::config::ShahConfig;
use crate::db::entity::{EntityCount, EntityKochFrom};
use crate::models::{Gene, GeneId};
use crate::{PAGE_SIZE, SystemError};

impl<
    Dk: Duck + EntityKochFrom<DkO, DkS>,
    Pn: Pond + EntityKochFrom<PnO, PnS>,
    Og: Origin + EntityKochFrom<OgO, OgS>,
    DkO: Duck,
    PnO: Pond,
    OgO: Origin,
    DkS,
    PnS,
    OgS,
> PondDb<Dk, Pn, Og, DkO, PnO, OgO, DkS, PnS, OgS>
{
    pub fn add(
        &mut self, origene: &Gene, item: &mut Dk,
    ) -> Result<(), ShahError> {
        item.set_alive(true);

        let mut origin = Og::default();
        self.origin.get(origene, &mut origin)?;
        *origin.item_count_mut() += 1;

        let mut pond = self.half_empty_pond(&mut origin)?;
        *pond.alive_mut() += 1;

        let mut buf = [Dk::default(); PAGE_SIZE];
        *item.pond_mut() = *pond.gene();
        *item.growth_mut() = 0;
        let ig = item.gene_mut();
        ig.server = ShahConfig::get().server;
        crate::utils::getrandom(&mut ig.pepper);

        let stack = if pond.stack() == 0 {
            let stack = self.new_stack_id()?;
            for (idx, x) in buf.iter_mut().enumerate() {
                let xg = x.gene_mut();
                xg.id = stack + idx as u64;
                xg.server = ig.server;
                *x.pond_mut() = *pond.gene();
            }
            ig.id = stack;
            ig.iter = 0;
            buf[0] = *item;

            *pond.stack_mut() = stack;
            *pond.empty_mut() = PAGE_SIZE as u8 - 1;
            stack
        } else {
            self.item.list(pond.stack(), &mut buf)?;

            let mut found_empty_slot = false;
            for (x, slot) in buf.iter_mut().enumerate() {
                let sg = slot.gene();
                if !slot.is_alive() && !sg.exhausted() {
                    let ig = item.gene_mut();
                    ig.id = pond.stack() + x as u64;
                    if sg.id != 0 {
                        ig.iter = sg.iter + 1;
                        *item.growth_mut() = slot.growth() + 1;
                    } else {
                        ig.iter = 0;
                    }
                    slot.clone_from(item);
                    found_empty_slot = true;
                    *pond.empty_mut() = pond.empty().saturating_sub(1);
                    // if pond.empty() > 0 {
                    //     pond.empty -= 1;
                    // }
                    break;
                }
            }
            if !found_empty_slot {
                log::error!("could not found an empty slot for item");
                return Err(SystemError::PondNoEmptySlotWasFound)?;
            }

            pond.stack()
        };

        self.item.write_buf_at(&buf, stack)?;
        self.pond.set(&mut pond)?;
        self.origin.set(&mut origin)?;
        self.item.live += 1;

        Ok(())
    }

    pub fn get(
        &mut self, gene: &Gene, entity: &mut Dk,
    ) -> Result<(), ShahError> {
        self.item.get(gene, entity)
    }

    pub fn count(&mut self) -> Result<EntityCount, ShahError> {
        self.item.count()
    }

    pub fn set(&mut self, entity: &mut Dk) -> Result<(), ShahError> {
        if !entity.is_alive() {
            log::error!("{} deleting using the set method", self.ls);
            return Err(SystemError::DeadSet)?;
        }

        let mut old_entity = Dk::default();
        self.item.get(entity.gene(), &mut old_entity)?;

        *entity.growth_mut() = old_entity.growth();
        *entity.pond_mut() = *old_entity.pond();
        self.item.set_unchecked(entity)?;

        Ok(())
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut Dk,
    ) -> Result<(), ShahError> {
        self.item.del(gene, entity)?;

        let mut pond = Pn::default();
        let mut origin = Og::default();

        self.pond.get(entity.pond(), &mut pond)?;
        *pond.alive_mut() = pond.alive().saturating_sub(1);
        // if pond.alive() > 0 {
        //     pond.alive -= 1;
        // }

        self.origin.get(pond.origin(), &mut origin)?;
        *origin.item_count_mut() = origin.item_count().saturating_sub(1);
        // if origin.items > 0 {
        //     origin.items -= 1;
        // }

        if pond.alive() == 0 {
            self.add_empty_pond(&mut origin, pond)?;
        } else {
            self.pond.set(&mut pond)?;
        }

        self.origin.set(&mut origin)?;

        Ok(())
    }

    pub fn list(
        &mut self, id: GeneId, result: &mut [Dk; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        self.item.list(id, result)
    }
}
