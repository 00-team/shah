use super::*;
use crate::ShahError;
use crate::db::entity::{ENTITY_META, EntityKochFrom};
use crate::models::{Binary, Gene, GeneId};
use crate::{OptNotFound, PAGE_SIZE};

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
    fn take_free(&mut self) -> Option<Gene> {
        self.free_list.pop(|_| true)
    }

    pub(super) fn add_empty_pond(
        &mut self, origin: &mut Og, mut pond: Pn,
    ) -> Result<(), ShahError> {
        *origin.pond_count_mut() = origin.pond_count().saturating_sub(1);
        // if *origin.pond_count() > 0 {
        //     origin.pond_ -= 1;
        // }

        let mut buf = [Dk::default(); PAGE_SIZE];
        self.item.list(pond.stack(), &mut buf)?;

        *pond.empty_mut() = 0;
        *pond.alive_mut() = 0;
        for item in buf {
            if !item.gene().exhausted() {
                *pond.empty_mut() += 1;
            }
            if item.is_alive() {
                log::warn!("adding a non-free pond to free_list");
                return Ok(());
            }
        }

        if origin.head() == pond.gene() {
            *origin.head_mut() = *pond.next();
        }

        if origin.tail() == pond.gene() {
            *origin.tail_mut() = *pond.past();
        }

        let mut old_pond = Pn::default();

        if let Err(e) = self.pond.get(pond.past(), &mut old_pond) {
            e.not_found_ok()?;
        } else {
            *old_pond.next_mut() = *pond.next();
            self.pond.set_unchecked(&mut old_pond)?;
        }

        if let Err(e) = self.pond.get(pond.next(), &mut old_pond) {
            e.not_found_ok()?;
        } else {
            *old_pond.past_mut() = *pond.past();
            self.pond.set_unchecked(&mut old_pond)?;
        }

        pond.next_mut().zeroed();
        pond.past_mut().zeroed();
        pond.origin_mut().zeroed();
        // *pond.set_is_free(true);
        self.pond.set(&mut pond)?;
        self.free_list.push(*pond.gene());
        Ok(())
    }

    pub(super) fn half_empty_pond(
        &mut self, origin: &mut Og,
    ) -> Result<Pn, ShahError> {
        let mut pond_gene = *origin.head();
        let mut pond = Pn::default();
        loop {
            if self.pond.get(&pond_gene, &mut pond).onf()?.is_none() {
                break;
            }

            if pond.empty() > 0 {
                return Ok(pond);
            }
            pond_gene = *pond.next();
        }

        let mut new = Pn::default();
        let add_new = if let Some(free) = self.take_free() {
            self.pond.get(&free, &mut new).onf()?.is_none()
        } else {
            true
        };

        if add_new {
            new.gene_mut().clear();
            self.pond.add(&mut new)?;
        }
        new.next_mut().clear();
        *new.alive_mut() = 0;
        *new.origin_mut() = *origin.gene();
        // new.set_is_free(false);

        *origin.pond_count_mut() += 1;

        if pond.is_alive() {
            *pond.next_mut() = *new.gene();
            *new.past_mut() = *origin.tail();
            *origin.tail_mut() = *new.gene();
            self.pond.set(&mut pond)?;
        } else {
            new.past_mut().clear();
            *origin.head_mut() = *new.gene();
            *origin.tail_mut() = *new.gene();
        }

        Ok(new)
    }

    pub(super) fn new_stack_id(&mut self) -> Result<GeneId, ShahError> {
        let pos = self.item.file_size()?;
        if pos < ENTITY_META + Dk::N {
            return Ok(GeneId(1));
        }

        let sn = Dk::N * PAGE_SIZE as u64;
        let usabe = pos - (ENTITY_META + Dk::N);

        let (id, offset) = (usabe / sn, usabe % sn);
        if offset != 0 {
            log::warn!("{} new-stack-id bad offset: {offset}", self.ls);
        }

        Ok(GeneId(id * PAGE_SIZE as u64 + 1))
    }
}
