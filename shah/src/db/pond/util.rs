use super::*;

impl<T: PondItem + EntityKochFrom<O, S>, O: EntityItem, S> PondDb<T, O, S> {
    fn take_free(&mut self) -> Option<Gene> {
        self.free_list.pop(|_| true)
    }

    pub(super) fn add_empty_pond(
        &mut self, origin: &mut Origin, mut pond: Pond,
    ) -> Result<(), ShahError> {
        if origin.ponds > 0 {
            origin.ponds -= 1;
        }

        let mut buf = [T::default(); PAGE_SIZE];
        self.items.list(pond.stack, &mut buf)?;

        pond.empty = 0;
        pond.alive = 0;
        for item in buf {
            if !item.gene().exhausted() {
                pond.empty += 1;
            }
            if item.is_alive() {
                log::warn!("adding a non-free pond to free_list");
                return Ok(());
            }
        }

        if origin.head == pond.gene {
            origin.head = pond.next;
        }

        if origin.tail == pond.gene {
            origin.tail = pond.past;
        }

        let mut old_pond = Pond::default();

        if let Err(e) = self.index.get(&pond.past, &mut old_pond) {
            e.not_found_ok()?;
        } else {
            old_pond.next = pond.next;
            self.index.set_unchecked(&mut old_pond)?;
        }

        if let Err(e) = self.index.get(&pond.next, &mut old_pond) {
            e.not_found_ok()?;
        } else {
            old_pond.past = pond.past;
            self.index.set_unchecked(&mut old_pond)?;
        }

        pond.next.zeroed();
        pond.past.zeroed();
        pond.origin.zeroed();
        pond.set_is_free(true);
        self.index.set(&mut pond)?;
        self.free_list.push(pond.gene);
        Ok(())
    }

    pub(super) fn half_empty_pond(
        &mut self, origin: &mut Origin,
    ) -> Result<Pond, ShahError> {
        let mut pond_gene = origin.head;
        let mut pond = Pond::default();
        loop {
            if self.index.get(&pond_gene, &mut pond).onf()?.is_none() {
                break;
            }

            if pond.empty > 0 {
                return Ok(pond);
            }
            pond_gene = pond.next;
        }

        let mut new = Pond::default();
        let add_new = if let Some(free) = self.take_free() {
            self.index.get(&free, &mut new).onf()?.is_none()
        } else {
            true
        };

        if add_new {
            new.gene.clear();
            self.index.add(&mut new)?;
        }
        new.next.clear();
        new.alive = 0;
        new.origin = origin.gene;
        new.set_is_free(false);

        origin.ponds += 1;

        if pond.is_alive() {
            pond.next = new.gene;
            new.past = origin.tail;
            origin.tail = new.gene;
            self.index.set(&mut pond)?;
        } else {
            new.past.clear();
            origin.head = new.gene;
            origin.tail = new.gene;
        }

        Ok(new)
    }

    pub(super) fn new_stack_id(&mut self) -> Result<GeneId, ShahError> {
        let pos = self.items.file_size()?;
        if pos < ENTITY_META + T::N {
            return Ok(GeneId(1));
        }

        let sn = T::N * PAGE_SIZE as u64;
        let usabe = pos - (ENTITY_META + T::N);

        let (id, offset) = (usabe / sn, usabe % sn);
        if offset != 0 {
            log::warn!("{} new-stack-id bad offset: {offset}", self.ls);
        }

        Ok(GeneId(id * PAGE_SIZE as u64 + 1))
    }
}
