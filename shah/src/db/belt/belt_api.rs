use super::*;

impl<B: Belt + EntityKochFrom<OB, BS>, OB: Belt, BS> BeltDb<B, OB, BS> {
    pub fn belt_add(
        &mut self, buckle_gene: &Gene, belt: &mut B,
    ) -> Result<(), ShahError> {
        belt.set_alive(true);

        let mut buckle = Buckle::default();
        self.buckle.get(buckle_gene, &mut buckle)?;

        *belt.buckle_mut() = buckle.gene;
        *belt.growth_mut() = 0;
        *belt.past_mut() = buckle.tail;
        belt.next_mut().clear();

        self.belt.add(belt)?;

        if buckle.head.is_none() {
            buckle.head = *belt.gene();
        }

        let old_tail_gene = buckle.tail;
        buckle.tail = *belt.gene();
        buckle.belts += 1;

        if self.belt.get(&old_tail_gene, belt).onf()?.is_some() {
            *belt.next_mut() = buckle.tail;
            self.belt.set(belt)?;
        }

        self.buckle.set(&mut buckle)
    }

    pub fn belt_get(
        &mut self, gene: &Gene, belt: &mut B,
    ) -> Result<(), ShahError> {
        self.belt.get(gene, belt)
    }

    pub fn belt_count(&mut self) -> Result<EntityCount, ShahError> {
        self.belt.count()
    }

    pub fn belt_set(&mut self, belt: &mut B) -> Result<(), ShahError> {
        if !belt.is_alive() {
            log::error!("{} DeadSet: using set to delete", self.ls);
            return Err(SystemError::DeadSet)?;
        }

        let mut old_belt = B::default();
        self.belt.get(belt.gene(), &mut old_belt)?;

        *belt.growth_mut() = old_belt.growth();
        *belt.next_mut() = *old_belt.next();
        *belt.past_mut() = *old_belt.past();
        *belt.buckle_mut() = *old_belt.buckle();

        self.belt.set_unchecked(belt)
    }

    pub fn belt_del(
        &mut self, gene: &Gene, belt: &mut B,
    ) -> Result<(), ShahError> {
        self.belt.get(gene, belt)?;

        let mut buckle = Buckle::default();
        self.buckle.get(belt.buckle(), &mut buckle)?;

        if buckle.belts > 0 {
            buckle.belts -= 1;
        }

        if buckle.head == *belt.gene() {
            buckle.head = *belt.next();
        }

        if buckle.tail == *belt.gene() {
            buckle.tail = *belt.past();
        }

        let mut sibling = B::default();

        if let Err(e) = self.belt.get(belt.past(), &mut sibling) {
            e.not_found_ok()?;
        } else {
            *sibling.next_mut() = *belt.next();
            self.belt.set_unchecked(&mut sibling)?;
        }

        if let Err(e) = self.belt.get(belt.next(), &mut sibling) {
            e.not_found_ok()?;
        } else {
            *sibling.past_mut() = *belt.past();
            self.belt.set_unchecked(&mut sibling)?;
        }

        self.belt.del_unchecked(belt)?;

        self.buckle.set_unchecked(&mut buckle)
    }

    pub fn belt_list(
        &mut self, page: GeneId, result: &mut [B],
    ) -> Result<usize, ShahError> {
        self.belt.list(page, result)
    }

    /// put the head as tail and return the head
    pub fn recycle(&mut self, _recycled: &mut B) -> Result<(), ShahError> {
        todo!("make this")
    }
}
