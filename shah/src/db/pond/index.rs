use super::{Duck, Origin, Pond, PondDb};
use crate::PAGE_SIZE;
use crate::db::entity::EntityKochFrom;
use crate::models::Gene;
use crate::{IsNotFound, ShahError};
use std::ops::AddAssign;

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
    pub fn pond_list(
        &mut self, pond: &mut Pn, result: &mut [Dk; PAGE_SIZE],
    ) -> Result<(), ShahError> {
        let pond_gene = *pond.gene();
        self.index.get(&pond_gene, pond)?;
        self.items.list(*pond.stack(), result)?;
        Ok(())
    }

    pub fn pond_free(&mut self, pond: &mut Pn) -> Result<(), ShahError> {
        let mut buf = [Dk::default(); PAGE_SIZE];
        self.items.list(*pond.stack(), &mut buf)?;

        *pond.empty_mut() = 0;
        for item in buf.iter_mut() {
            if item.is_alive() {
                item.growth_mut().add_assign(1);
                item.set_alive(false);
            }
            if !item.gene().exhausted() {
                *pond.empty_mut() += 1;
            }
        }

        self.items.write_buf_at(&buf, *pond.stack())?;

        // pond.set_is_free(true);
        *pond.alive_mut() = 0;

        self.index.set(pond)?;
        self.free_list.push(*pond.gene());

        Ok(())
    }

    pub fn cascade(&mut self, origene: &Gene) -> Result<(), ShahError> {
        let mut origin = Og::default();
        self.origins.get(origene, &mut origin)?;

        let mut pond_gene = *origin.head();
        let mut pond = Pn::default();
        loop {
            if let Err(e) = self.index.get(&pond_gene, &mut pond) {
                if e.is_not_found() {
                    break;
                }
                return Err(e)?;
            }
            pond_gene = *pond.next();
            self.pond_free(&mut pond)?;
        }

        self.origins.del(origene, &mut origin)?;

        Ok(())
    }
}
