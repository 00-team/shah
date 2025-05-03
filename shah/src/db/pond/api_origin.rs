use super::{Duck, Origin, Pond, PondDb};
use crate::db::derr;
use crate::db::entity::EntityKochFrom;
use crate::models::Gene;
use crate::{IsNotFound, ShahError};
use crate::{OptNotFound, SystemError};

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
    pub fn origin_root(&mut self) -> Result<(), ShahError> {
        let mut origin = Og::default();
        if self.origin.get(&Gene::ROOT, &mut origin).onf()?.is_none() {
            origin.zeroed();
            origin.set_alive(true);
            *origin.gene_mut() = Gene::ROOT;
            self.origin.set_unchecked(&mut origin)?;
        }

        Ok(())
    }

    pub fn origin_get(
        &mut self, gene: &Gene, origin: &mut Og,
    ) -> Result<(), ShahError> {
        self.origin.get(gene, origin)
    }

    pub fn origin_init(
        &mut self, gene: &Gene, origin: &mut Og,
    ) -> Result<(), ShahError> {
        let def = *origin;
        if gene.is_none() || self.origin.get(gene, origin).onf()?.is_none() {
            origin.clone_from(&def);
            origin.head_mut().clear();
            origin.tail_mut().clear();
            *origin.pond_count_mut() = 0;
            *origin.item_count_mut() = 0;
            origin.gene_mut().clear();
            self.origin.add(origin)?;
        }

        Ok(())
    }

    pub fn origin_set(&mut self, origin: &mut Og) -> Result<(), ShahError> {
        if !origin.is_alive() {
            return derr!(self.ls, SystemError::DeadSet);
        }

        let mut old = Og::default();
        self.origin.get(origin.gene(), &mut old)?;

        *origin.growth_mut() = old.growth();
        *origin.head_mut() = *old.head();
        *origin.tail_mut() = *old.tail();
        *origin.pond_count_mut() = old.pond_count();
        *origin.item_count_mut() = old.item_count();

        self.origin.set_unchecked(origin)?;

        Ok(())
    }

    pub fn origin_del(&mut self, gene: &Gene) -> Result<(), ShahError> {
        let mut origin = Og::default();
        self.origin.get(gene, &mut origin)?;

        let mut pond_gene = *origin.head();
        let mut pond = Pn::default();
        loop {
            if let Err(e) = self.pond.get(&pond_gene, &mut pond) {
                if e.is_not_found() {
                    break;
                }
                return Err(e)?;
            }
            pond_gene = *pond.next();
            self.pond_free(&mut pond)?;
        }

        self.origin.del(gene, &mut origin)?;

        Ok(())
    }
}
