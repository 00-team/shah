mod api;
mod coords;

pub use self::coords::ApexCoords;

use self::coords::MAX_ZOOM;
use super::entity::EntityDb;
use crate::{
    ShahError,
    config::ShahConfig,
    db::entity::EntityFlags,
    models::{Gene, Performed, Task, TaskList, Worker},
    utils,
};

pub trait ApexTileData: crate::ShahModel + crate::models::ShahSchema {
    fn is_some(&self) -> bool;
    fn is_none(&self) -> bool;
    fn gene(&self) -> &Gene;
    fn gene_mut(&mut self) -> &mut Gene;
    fn new(gene: Gene) -> Self {
        let mut d = Self::default();
        d.gene_mut().clone_from(&gene);
        d
    }
    fn clear(&mut self);
}

impl ApexTileData for Gene {
    fn gene(&self) -> &Gene {
        self
    }
    fn is_some(&self) -> bool {
        self.is_some()
    }
    fn is_none(&self) -> bool {
        self.is_none()
    }
    fn gene_mut(&mut self) -> &mut Gene {
        self
    }
    fn new(gene: Gene) -> Self {
        gene
    }
    fn clear(&mut self) {
        self.clear();
    }
}

#[shah::model]
#[derive(Debug, shah::Entity, shah::ShahSchema)]
struct ApexTile<const S: usize, D: ApexTileData> {
    gene: Gene,
    growth: u64,
    entity_flags: EntityFlags,
    level: u8, // 0 - 6 - 12
    _pad: [u8; 6],
    tiles: [D; S],
}

#[derive(Debug)]
pub struct ApexDb<
    const LVL: usize,
    const LEN: usize,
    const SIZ: usize,
    D: ApexTileData,
> {
    tiles: EntityDb<ApexTile<SIZ, D>>,
    tasks: TaskList<1, Task<Self>>,
    root: Gene,
}

impl<const LVL: usize, const LEN: usize, const SIZ: usize, D: ApexTileData>
    ApexDb<LVL, LEN, SIZ, D>
{
    pub fn new(path: &str) -> Result<Self, ShahError> {
        assert!(LVL > 0, "LVL must be at least 1");
        assert!(LVL <= 6, "LVL must be at most 6");
        assert!(LEN > 0, "LEN must be at least 1");
        assert!(LVL * LEN < MAX_ZOOM, "LVL * LEN must be at most {MAX_ZOOM}");
        assert_eq!(
            1 << (LVL * 2),
            SIZ,
            "SIZ must be equal to: {}",
            1 << (LVL * 2)
        );
        ApexTile::<SIZ, D>::__assert_padding();

        let conf = ShahConfig::get();
        let data_path = conf.data_dir.join(path);
        let name = data_path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("could not get file_name from path");

        utils::validate_db_name(name)?;

        std::fs::create_dir_all(&data_path)?;

        let db = Self {
            tiles: EntityDb::new(&format!("{path}/apex"), 0)?,
            tasks: TaskList::new([Self::work_tiles]),
            root: Gene::keyed(1, [59, 77, 69]),
        };

        Ok(db)
    }

    fn work_tiles(&mut self) -> Result<Performed, ShahError> {
        self.tiles.work()
    }

    // pub fn work(&mut self) -> Result<Performed, ShahError> {
    //     self.tasks.start();
    //     while let Some(task) = self.tasks.next() {
    //         if task(self)?.0 {
    //             return Ok(Performed(true));
    //         }
    //     }
    //     Ok(Performed(false))
    // }

    fn add(&mut self, tree: &[usize], value: D) -> Result<D, ShahError> {
        let mut data = value;
        for (i, x) in tree.iter().rev().enumerate() {
            let mut tile = ApexTile::<SIZ, D>::default();
            tile.tiles[*x] = data;
            tile.level = ((LEN - i - 1) * LVL) as u8;
            self.tiles.add(&mut tile)?;
            *data.gene_mut() = tile.gene;
        }
        Ok(data)
    }
}

impl<const LVL: usize, const LEN: usize, const SIZ: usize, D: ApexTileData>
    Worker<1> for ApexDb<LVL, LEN, SIZ, D>
{
    fn tasks(&mut self) -> &mut TaskList<1, Task<Self>> {
        &mut self.tasks
    }
}
