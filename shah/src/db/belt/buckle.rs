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
    pub fn buckle_root(&mut self) -> Result<(), ShahError> {
        let mut buckle = Bk::default();
        if self.buckle.get(&Gene::ROOT, &mut buckle).onf()?.is_none() {
            buckle.zeroed();
            buckle.set_alive(true);
            *buckle.gene_mut() = Gene::ROOT;
            self.buckle.set_unchecked(&mut buckle)?;
        }

        Ok(())
    }

    pub fn buckle_add(&mut self, buckle: &mut Bk) -> Result<(), ShahError> {
        buckle.set_alive(true);
        *buckle.belt_count_mut() = 0;
        *buckle.growth_mut() = 0;
        buckle.head_mut().clear();
        buckle.tail_mut().clear();

        self.buckle.add(buckle)
    }

    pub fn buckle_get(
        &mut self, gene: &Gene, buckle: &mut Bk,
    ) -> Result<(), ShahError> {
        self.buckle.get(gene, buckle)
    }

    pub fn buckle_count(&mut self) -> Result<EntityCount, ShahError> {
        self.buckle.count()
    }

    pub fn buckle_list(
        &mut self, page: GeneId, result: &mut [Bk; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        self.buckle.list(page, result)
    }

    /// this will cascade all the belts under this buckle
    pub fn buckle_del(&mut self, gene: &Gene) -> Result<(), ShahError> {
        let mut buckle = Bk::default();
        self.buckle.get(gene, &mut buckle)?;

        let mut belt_gene = *buckle.head();
        let mut belt = Bt::default();
        loop {
            if let Err(e) = self.belt.del(&belt_gene, &mut belt) {
                if e.is_not_found() {
                    break;
                }
                return Err(e)?;
            }
            belt_gene = *belt.next();
        }

        self.buckle.del_unchecked(&mut buckle)
    }
}
