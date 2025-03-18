use super::*;

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub fn get(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        gene.validate()?;

        self.read_at(entity, gene.id)?;

        let egene = entity.gene();
        if egene.id == 0 {
            if let Some(koch) = self.koch.as_mut() {
                let mut oldie = koch.get(gene)?;
                self.set_unchecked(&mut oldie)?;
                if !oldie.is_alive() {
                    self.add_dead(oldie.gene());
                    return Err(NotFound::EntityNotAlive)?;
                }
                entity.clone_from(&oldie);
                oldie.gene().check(gene)?;
                return Ok(());
            }

            log::error!("{} get: gene id mismatch", self.ls);
            return Err(SystemError::GeneIdMismatch)?;
        }
        egene.check(gene)?;

        if !entity.is_alive() {
            return Err(NotFound::EntityNotAlive)?;
        }

        Ok(())
    }

    pub fn add(&mut self, entity: &mut T) -> Result<(), ShahError> {
        entity.set_alive(true);
        let gene = entity.gene_mut();
        if gene.is_some() {
            log::warn!("{} entity gene is not cleared: {gene:?}", self.ls);
        }
        gene.clone_from(&self.new_gene()?);

        *entity.growth_mut() = 0;
        self.set_unchecked(entity)?;
        self.live += 1;

        Ok(())
    }

    pub fn count(&mut self) -> Result<EntityCount, ShahError> {
        Ok(EntityCount { total: self.total()?, alive: self.live })
    }

    pub fn set(&mut self, entity: &mut T) -> Result<(), ShahError> {
        if !entity.is_alive() {
            log::error!("{} deleteing entity using the set method", self.ls);
            return Err(SystemError::DeadSet)?;
        }

        let mut old_entity = T::default();
        self.get(entity.gene(), &mut old_entity)?;
        // let growth = old_entity.growth();
        // let gene = old_entity.gene().clone();
        // old_entity.clone_from(&entity);
        // *old_entity.growth_mut() = growth;
        // old_entity.gene_mut().clone_from(&gene);

        *entity.growth_mut() = old_entity.growth();
        self.set_unchecked(entity)
    }

    pub fn del(
        &mut self, gene: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        // first make sure that the entity is alive and exists
        self.get(gene, entity)?;
        // then delete unchecked
        self.del_unchecked(entity)
    }

    pub fn list(
        &mut self, page: GeneId, result: &mut [T; PAGE_SIZE],
    ) -> Result<usize, ShahError> {
        let pos = Self::id_to_pos(page * PAGE_SIZE as u64 + 1);
        let size = self.file.read_at(result.as_binary_mut(), pos)?;
        let count = size / T::S;
        if count != PAGE_SIZE {
            for item in result.iter_mut().skip(count) {
                item.zeroed()
            }
        }

        Ok(count)
    }
}
