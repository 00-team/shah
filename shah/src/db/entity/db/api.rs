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
                    self.dead_add(oldie.gene());
                    return Err(NotFound::EntityNotAlive)?;
                }
                entity.clone_from(&oldie);
                oldie.gene().check(gene, &self.ls)?;
                return Ok(());
            }

            log::error!("{} get: gene id 0 != {:?}", self.ls, gene);
            return Err(SystemError::GeneIdMismatch)?;
        }
        egene.check(gene, &self.ls)?;

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
        *gene = self.new_gene()?;

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

    pub fn keyed(
        &mut self, key: &Gene, entity: &mut T,
    ) -> Result<(), ShahError> {
        if self.get(key, entity).onf()?.is_none() {
            entity.zeroed();
            entity.set_alive(true);
            *entity.gene_mut() = *key;
            self.set_unchecked(entity)?;
        }

        Ok(())
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
        &mut self, id: GeneId, result: &mut [T],
    ) -> Result<usize, ShahError> {
        if id == 0 {
            return Err(NotFound::ListIdZero)?;
        }

        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                result as *mut [T] as *mut u8,
                result.len() * T::S,
            )
        };

        let pos = Self::id_to_pos(id);
        let size = self.file.read_at(buf, pos)?;
        if size < T::S {
            return Err(NotFound::OutOfBounds)?;
        }
        let count = size / T::S;

        for (idx, item) in result.iter_mut().enumerate() {
            if idx >= count {
                item.zeroed();
                continue;
            }

            let egene = item.gene();
            if egene.id == 0 {
                let Some(koch) = self.koch.as_mut() else {
                    item.zeroed();
                    continue;
                    // log::error!(
                    //     "{} list: item.gene.id == 0 and there is not koch",
                    //     self.ls
                    // );
                    // return Err(DbError::NoKoch)?;
                };

                let Some(mut old) = koch.get_id(id + idx as u64).onf()? else {
                    item.zeroed();
                    continue;
                };

                self.set_unchecked(&mut old)?;
                if !old.is_alive() {
                    self.dead_add(old.gene());
                }
                item.clone_from(&old);
            }
        }

        Ok(count)
    }
}
