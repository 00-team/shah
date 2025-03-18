use super::*;

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub(crate) fn file_size(&mut self) -> std::io::Result<u64> {
        self.file.seek(SeekFrom::End(0))
    }

    pub(super) fn total(&mut self) -> Result<GeneId, ShahError> {
        let file_size = self.file_size()?;
        if file_size < ENTITY_META {
            log::warn!("{} total file_size is less than ENTITY_META", self.ls);
            return Ok(GeneId(0));
        }
        if file_size < ENTITY_META + T::N {
            log::warn!(
                "{} total file_size is less than ENTITY_META + T::N",
                self.ls
            );
            return Ok(GeneId(0));
        }

        Ok(GeneId((file_size - ENTITY_META) / T::N - 1))
    }

    pub(super) fn id_to_pos(id: GeneId) -> u64 {
        ENTITY_META + (id * T::N).0
    }

    pub(crate) fn write_buf_at<B: Binary>(
        &self, buf: &B, id: GeneId,
    ) -> Result<(), ShahError> {
        let pos = Self::id_to_pos(id);
        self.file.write_all_at(buf.as_binary(), pos)?;
        Ok(())
    }

    pub(crate) fn read_buf_at<B: Binary>(
        &self, buf: &mut B, id: GeneId,
    ) -> Result<(), ShahError> {
        let pos = Self::id_to_pos(id);
        match self.file.read_exact_at(buf.as_binary_mut(), pos) {
            Ok(_) => Ok(()),
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => Err(NotFound::OutOfBounds)?,
                _ => {
                    log::error!("{} read_buf_at: {e:?}", self.ls);
                    Err(e)?
                }
            },
        }
    }

    pub(crate) fn read_at(
        &self, entity: &mut T, id: GeneId,
    ) -> Result<(), ShahError> {
        self.read_buf_at(entity, id)
    }

    pub(crate) fn del_unchecked(
        &mut self, entity: &mut T,
    ) -> Result<(), ShahError> {
        entity.set_alive(false);
        self.set_unchecked(entity)?;
        self.add_dead(entity.gene());
        Ok(())
    }

    pub(crate) fn set_unchecked(
        &mut self, entity: &mut T,
    ) -> Result<(), ShahError> {
        entity.growth_mut().add_assign(1);
        self.write_buf_at(entity, entity.gene().id)?;
        Ok(())
    }

    pub(super) fn new_gene_id(&mut self) -> Result<GeneId, ShahError> {
        let pos = self.file.seek(SeekFrom::End(0))?;
        if pos < ENTITY_META + T::N {
            return Ok(GeneId(1));
        }

        let db_pos = pos - ENTITY_META;
        let (id, offset) = (db_pos / T::N, db_pos % T::N);
        if offset != 0 {
            log::warn!(
                "{} id: {id} | new-gene-id bad offset: {offset}",
                self.ls
            );
        }

        Ok(GeneId(id))
    }

    pub(super) fn new_gene(&mut self) -> Result<Gene, ShahError> {
        let mut gene = Gene { id: self.take_dead_id(), ..Default::default() };
        utils::getrandom(&mut gene.pepper);
        gene.server = 0;
        gene.iter = 0;

        if gene.id != 0 {
            let mut old = T::default();
            if self.read_at(&mut old, gene.id).is_ok() {
                let og = old.gene();
                if !og.exhausted() {
                    gene.iter = og.iter + 1;
                    return Ok(gene);
                }
            }
        }

        gene.id = self.new_gene_id()?;

        Ok(gene)
    }
}
