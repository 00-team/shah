mod api;
mod coords;

pub use self::coords::ApexCoords;

use self::coords::MAX_ZOOM;
use super::entity::EntityDb;
use crate::{
    ShahError,
    models::{Gene, Performed, Task, TaskList, Worker},
    utils,
};

#[derive(shah::ShahSchema)]
#[shah::model]
#[derive(Debug, shah::Entity)]
struct ApexTile<const S: usize> {
    gene: Gene,
    growth: u64,
    entity_flags: u8,
    level: u8, // 0 - 6 - 12
    _pad: [u8; 6],
    tiles: [Gene; S],
}

#[derive(Debug)]
pub struct ApexDb<const LVL: usize, const LEN: usize, const SIZ: usize> {
    tiles: EntityDb<ApexTile<SIZ>>,
    tasks: TaskList<1, Task<Self>>,
    root: Gene,
}

impl<const LVL: usize, const LEN: usize, const SIZ: usize>
    ApexDb<LVL, LEN, SIZ>
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
        ApexTile::<SIZ>::__assert_padding();

        let data_path = std::path::Path::new("data/").join(path);
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

    fn add(&mut self, tree: &[usize], value: Gene) -> Result<Gene, ShahError> {
        let mut gene = value;
        for (i, x) in tree.iter().rev().enumerate() {
            let mut tile = ApexTile::<SIZ>::default();
            tile.tiles[*x] = gene;
            tile.level = ((LEN - i - 1) * LVL) as u8;
            self.tiles.add(&mut tile)?;
            gene = tile.gene;
        }
        Ok(gene)
    }
}

impl<const LVL: usize, const LEN: usize, const SIZ: usize> Worker<1>
    for ApexDb<LVL, LEN, SIZ>
{
    fn tasks(&mut self) -> &mut TaskList<1, Task<Self>> {
        &mut self.tasks
    }
}
