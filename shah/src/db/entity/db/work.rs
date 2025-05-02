use super::*;

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub(super) fn inspection(&mut self, entity: &T) {
        // log::debug!("\x1b[36minspecting\x1b[m: {:?}", entity.gene());

        if !entity.is_alive() {
            let gene = entity.gene();
            self.dead_add(gene);
            log::debug!("{} found dead: {} | {}", self.ls, gene.id, self.live);
        }

        if let Some(ei) = &self.inspector {
            if let Err(e) = ei.call(entity) {
                log::error!("{} inspection failed: {e:#?}", self.ls);
            }
        }
    }

    pub(super) fn work_setup_task(&mut self) -> Result<Performed, ShahError> {
        if self.setup_prog.ended() {
            return Ok(Performed(false));
        }

        let mut entity = T::default();
        let mut performed = false;
        for _ in 0..10 {
            let Some(id) = self.setup_prog.next() else { break };
            performed = true;

            if self.read_at(&mut entity, id).onf()?.is_none() {
                self.setup_prog.end();
                log::warn!(
                    "{} work_setup_task read_at not found {id:?}",
                    self.ls
                );
                break;
            }

            self.inspection(&entity);
        }

        Ok(Performed(performed))
    }

    pub(super) fn koch_prog_get(&mut self) -> Result<(), ShahError> {
        let buf = self.koch_prog.as_binary_mut();
        if let Err(e) = self.file.read_exact_at(buf, EntityHead::N) {
            if e.kind() != ErrorKind::UnexpectedEof {
                log::error!("{} read error: {e:?}", self.ls);
                return Err(e)?;
            }

            self.koch_prog = EntityKochProg::default();
            self.koch_prog_set()?;
        }

        Ok(())
    }

    fn koch_prog_set(&mut self) -> Result<(), ShahError> {
        self.file.write_all_at(self.koch_prog.as_binary(), EntityHead::N)?;
        Ok(())
    }

    pub(super) fn work_koch(&mut self) -> Result<Performed, ShahError> {
        if self.koch.is_none() || self.koch_prog.ended() {
            return Ok(Performed(false));
        }

        let mut current = T::default();
        let mut performed = false;
        for _ in 0..self.work_iter {
            let Some(id) = self.koch_prog.next() else { break };
            let Some(koch) = self.koch.as_mut() else { break };

            let old = match koch.get_id(id) {
                Ok(v) => v,
                Err(e) => {
                    if matches!(e, ShahError::NotFound(NotFound::EmptyItem)) {
                        performed = true;
                        continue;
                    }

                    log::warn!("{} koch.get_id({id:?}): {e:?}", self.ls);
                    e.not_found_ok()?;
                    self.koch_prog.end();
                    break;
                }
            };
            performed = true;

            if self.read_at(&mut current, id).is_ok()
                && old.growth() <= current.growth()
            {
                // if we already did koch and updated the item do not koch again
                continue;
            }

            self.write_buf_at(&old, id)?;
            self.inspection(&old);
            log::debug!("{} koched: {:?}", self.ls, old.gene());
        }

        if performed {
            self.koch_prog_set()?;
        }

        Ok(Performed(performed))
    }
}

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    Worker<2> for EntityDb<T, O, S, Is>
{
    fn tasks(&mut self) -> &mut TaskList<2, Task<Self>> {
        &mut self.tasks
    }
}
