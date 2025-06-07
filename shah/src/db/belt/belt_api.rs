use super::*;

impl<
    Bt: Belt + EntityKochFrom<BtO, BtS>,
    Bk: Buckle + EntityKochFrom<BkO, BkS>,
    BtO: Belt,
    BkO: Buckle,
    BtS,
    BkS,
> BeltDb<Bt, Bk, BtO, BkO, BtS, BkS>
{
    pub fn belt_add(
        &mut self, buckle_gene: &Gene, belt: &mut Bt,
    ) -> Result<(), ShahError> {
        let mut buckle = Bk::default();
        self.buckle.get(buckle_gene, &mut buckle)?;

        belt.set_alive(true);

        *belt.buckle_mut() = *buckle.gene();
        *belt.growth_mut() = 0;
        *belt.past_mut() = *buckle.tail();
        belt.next_mut().clear();

        self.belt.add(belt)?;

        if buckle.head().is_none() {
            *buckle.head_mut() = *belt.gene();
        }

        let mut sib = Bt::default();
        let old_tail_gene = *buckle.tail();
        *buckle.tail_mut() = *belt.gene();
        *buckle.belt_count_mut() += 1;

        if self.belt.get(&old_tail_gene, &mut sib).onf()?.is_some() {
            *sib.next_mut() = *buckle.tail();
            self.belt.set(&mut sib)?;
        }

        self.buckle.set(&mut buckle)
    }

    pub fn belt_add_bulk(
        &mut self, buckle_gene: &Gene, belts: &mut [Bt],
    ) -> Result<(), ShahError> {
        let mut buckle = Bk::default();
        self.buckle.get(buckle_gene, &mut buckle)?;

        for belt in belts {
            belt.set_alive(true);
            *belt.buckle_mut() = *buckle.gene();
            *belt.growth_mut() = 0;
            *belt.past_mut() = *buckle.tail();
            belt.next_mut().clear();

            self.belt.add(belt)?;
            if buckle.head().is_none() {
                *buckle.head_mut() = *belt.gene();
            }

            let old_tail_gene = *buckle.tail();
            *buckle.tail_mut() = *belt.gene();
            *buckle.belt_count_mut() += 1;

            let mut tail = Bt::default();
            if self.belt.get(&old_tail_gene, &mut tail).onf()?.is_some() {
                *tail.next_mut() = *buckle.tail();
                self.belt.set(&mut tail)?;
            }
        }

        self.buckle.set(&mut buckle)
    }

    pub fn belt_get(
        &mut self, gene: &Gene, belt: &mut Bt,
    ) -> Result<(), ShahError> {
        self.belt.get(gene, belt)
    }

    pub fn belt_count(&mut self) -> Result<EntityCount, ShahError> {
        self.belt.count()
    }

    pub fn belt_set(&mut self, belt: &mut Bt) -> Result<(), ShahError> {
        if !belt.is_alive() {
            log::error!("{} DeadSet: using set to delete", self.ls);
            return Err(SystemError::DeadSet)?;
        }

        let mut old_belt = Bt::default();
        self.belt.get(belt.gene(), &mut old_belt)?;

        *belt.growth_mut() = old_belt.growth();
        *belt.next_mut() = *old_belt.next();
        *belt.past_mut() = *old_belt.past();
        *belt.buckle_mut() = *old_belt.buckle();

        self.belt.set_unchecked(belt)
    }

    pub fn belt_del(
        &mut self, gene: &Gene, belt: &mut Bt,
    ) -> Result<(), ShahError> {
        self.belt.get(gene, belt)?;

        let mut buckle = Bk::default();
        self.buckle.get(belt.buckle(), &mut buckle)?;

        *buckle.belt_count_mut() = buckle.belt_count().saturating_sub(1);
        // if buckle.belts > 0 {
        //     buckle.belts -= 1;
        // }

        if buckle.head() == belt.gene() {
            *buckle.head_mut() = *belt.next();
        }

        if buckle.tail() == belt.gene() {
            *buckle.tail_mut() = *belt.past();
        }

        let mut sibling = Bt::default();

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
        &mut self, id: GeneId, result: &mut [Bt],
    ) -> Result<usize, ShahError> {
        self.belt.list(id, result)
    }

    pub fn move_to_tail(
        &mut self, gene: &Gene, belt: &mut Bt,
    ) -> Result<(), ShahError> {
        self.belt.get(gene, belt)?;
        let mut buckle = Bk::default();
        self.buckle.get(belt.buckle(), &mut buckle)?;

        if buckle.tail() == belt.gene() {
            return Ok(());
        }

        let mut old = Bt::default();

        if self.belt.get(belt.past(), &mut old).onf()?.is_some() {
            *old.next_mut() = *belt.next_mut();
            self.belt.set_unchecked(&mut old)?;
        }

        if self.belt.get(belt.next(), &mut old).onf()?.is_some() {
            if buckle.head() == belt.gene() {
                *buckle.head_mut() = *old.gene();
            }

            *old.past_mut() = *belt.past_mut();
            self.belt.set_unchecked(&mut old)?;
        }

        belt.next_mut().clear();
        belt.past_mut().clear();

        if self.belt.get(buckle.tail(), &mut old).onf()?.is_some() {
            *old.next_mut() = *belt.gene();
            *belt.past_mut() = *old.gene();
            self.belt.set_unchecked(&mut old)?;
        }

        *buckle.tail_mut() = *belt.gene();
        self.belt.set_unchecked(belt)?;
        self.buckle.set_unchecked(&mut buckle)?;

        Ok(())
    }
}
