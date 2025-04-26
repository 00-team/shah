use super::{Duck, Origin, Pond, PondDb};
use crate::PAGE_SIZE;
use crate::ShahError;
use crate::db::entity::EntityKochFrom;
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
        self.pond.get(&pond_gene, pond)?;
        self.item.list(pond.stack(), result)?;
        Ok(())
    }

    pub fn pond_free(&mut self, pond: &mut Pn) -> Result<(), ShahError> {
        let mut buf = [Dk::default(); PAGE_SIZE];
        self.item.list(pond.stack(), &mut buf)?;

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

        self.item.write_buf_at(&buf, pond.stack())?;

        // pond.set_is_free(true);
        *pond.alive_mut() = 0;

        self.pond.set(pond)?;
        self.free_list.push(*pond.gene());

        Ok(())
    }
}
