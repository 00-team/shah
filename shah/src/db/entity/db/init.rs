use super::*;

impl<S, T: EntityItem + EntityKochFrom<O, S>, O: EntityItem, Is: 'static>
    EntityDb<T, O, S, Is>
{
    pub fn new(path: &str, revision: u16) -> Result<Self, ShahError> {
        let data_path = Path::new("data/").join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path: {path}");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(data_path.join(format!("{name}.{revision}.shah")))?;

        let mut db = Self {
            live: GeneId(0),
            dead_list: DeadList::<GeneId, BLOCK_SIZE>::new(),
            file,
            revision,
            name: name.to_string(),
            koch: None,
            koch_prog: EntityKochProg::default(),
            setup_prog: SetupProg::default(),
            tasks: TaskList::new([Self::work_koch, Self::work_setup_task]),
            ls: format!("<EntityDb {path}.{revision} />"),
            inspector: None,
            work_iter: 10,
        };

        db.init()?;

        Ok(db)
    }

    fn init(&mut self) -> Result<(), ShahError> {
        self.init_head()?;
        self.koch_prog_get()?;

        self.live = GeneId(0);
        if !self.dead_list.disabled() {
            self.dead_list.clear();
        }

        let file_size = self.file_size()?;
        if file_size < ENTITY_META {
            log::error!("{} init somehow failed", self.ls);
            return Err(DbError::BadInit)?;
        }

        if file_size < ENTITY_META + T::N {
            self.file.write_all_at(T::default().as_binary(), ENTITY_META)?;
            return Ok(());
        }

        if file_size == ENTITY_META + T::N {
            return Ok(());
        }

        self.live = GeneId(((file_size - ENTITY_META) / T::N) - 1);

        self.setup_prog.prog = GeneId(1);
        self.setup_prog.total = self.live + 1;
        log::info!("{} init::setup_task {:?}", self.ls, self.setup_prog);

        Ok(())
    }

    fn init_head(&mut self) -> Result<(), ShahError> {
        let mut head = EntityHead::default();
        if let Err(e) = self.file.read_exact_at(head.as_binary_mut(), 0) {
            if e.kind() != ErrorKind::UnexpectedEof {
                log::error!("{} read error: {e:?}", self.ls);
                return Err(e)?;
            }

            head.db_head.init(
                ENTITY_MAGIC,
                self.revision,
                &self.name,
                ENTITY_VERSION,
            );

            head.item_size = T::N;

            let svec = T::shah_schema().encode();
            head.schema[0..svec.len()].clone_from_slice(&svec);

            self.file.write_all_at(head.as_binary(), 0)?;

            return Ok(());
        }

        head.check::<T>(self.revision, &self.ls)?;

        Ok(())
    }
}
