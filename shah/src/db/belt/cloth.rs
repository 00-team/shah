use super::BeltDb;
use crate::db::entity::Entity;
use crate::{AsUtf8Str, IsNotFound, OptNotFound, ShahError};
use crate::{ClientError, Taker, models::Gene};

#[derive(shah::ShahSchema)]
#[shah::model]
#[derive(Debug, shah::Entity, shah::Belt)]
pub struct ClothBelt<const S: usize> {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub buckle: Gene,
    growth: u64,
    pub length: u16,
    entity_flags: u8,
    #[flags(is_end)]
    flags: u8,
    _pad: [u8; 4],
    #[str]
    pub data: [u8; S],
}

#[derive(shah::ShahSchema)]
#[shah::model]
#[derive(Debug, shah::Entity, shah::Buckle)]
pub struct ClothBuckle {
    pub gene: Gene,
    pub head: Gene,
    pub tail: Gene,
    #[buckle(belt_count)]
    pub chunks: u64,
    pub owner: Gene,
    pub growth: u64,
    entity_flags: u8,
    _pad: [u8; 3],
    pub length: u32,
}

pub type BeltClothDb<const S: usize> = BeltDb<ClothBelt<S>, ClothBuckle>;

impl<const S: usize> BeltClothDb<S> {
    pub fn get(&mut self, bg: &Gene) -> Result<String, ShahError> {
        let mut buckle = ClothBuckle::default();
        self.buckle_get(bg, &mut buckle)?;

        let mut data = Vec::with_capacity(buckle.chunks as usize * S);

        let mut gene = buckle.head;
        loop {
            if gene.is_none() {
                break;
            }

            let mut cloth = ClothBelt::<S>::default();
            if self.belt_get(&gene, &mut cloth).onf()?.is_none() {
                break;
            }

            let len = (cloth.length as usize).min(cloth.data.len());
            data.extend_from_slice(&cloth.data[..len]);
            gene = cloth.next;
            if cloth.is_end() {
                data.push(0);
                break;
            }
        }

        Ok(data[..].as_utf8_str_null_terminated().to_string())
    }

    pub fn set(&mut self, bg: &Gene, data: &str) -> Result<(), ShahError> {
        let data = data.as_bytes();

        let mut buckle = ClothBuckle::default();
        self.buckle_get(bg, &mut buckle)?;
        buckle.length = data.len() as u32;
        self.buckle_set(&mut buckle)?;

        let mut gene = buckle.head;
        let mut cloth = ClothBelt::<S>::default();
        let mut it = data.chunks(S).peekable();

        loop {
            let Some(x) = it.next() else { break };
            cloth.data[x.len()..].fill(0);
            cloth.data[..x.len()].copy_from_slice(x);
            cloth.length = x.len() as u16;
            cloth.set_alive(true);
            cloth.set_is_end(it.peek().is_none());

            if gene.is_none() {
                cloth.gene.clear();
                self.belt_add(&buckle.gene, &mut cloth)?;
                continue;
            }

            cloth.gene = gene;
            match self.belt_set(&mut cloth).onf()? {
                Some(_) => gene = cloth.next,
                None => {
                    gene.clear();
                    cloth.gene.clear();
                    self.belt_add(&buckle.gene, &mut cloth)?;
                }
            }
        }

        Ok(())
    }
}

type C<Ok, E> = Result<Ok, ClientError<E>>;
pub struct ClothClient<E: IsNotFound + From<u16> + Copy, const S: usize> {
    pub buckle_get: fn(&Taker, &Gene) -> C<ClothBuckle, E>,
    pub buckle_set: fn(&Taker, &ClothBuckle) -> C<ClothBuckle, E>,
    pub belt_get: fn(&Taker, &Gene) -> C<ClothBelt<S>, E>,
    pub belt_set: fn(&Taker, &ClothBelt<S>) -> C<ClothBelt<S>, E>,
    pub belt_add: fn(&Taker, &Gene, &ClothBelt<S>) -> C<ClothBelt<S>, E>,
}

impl<E: IsNotFound + From<u16> + Copy, const S: usize> ClothClient<E, S> {
    pub fn get(
        &self, taker: &Taker, buckle_gene: &Gene,
    ) -> Result<String, ClientError<E>> {
        let buckle = (self.buckle_get)(taker, buckle_gene)?;
        let mut data = Vec::with_capacity(buckle.chunks as usize * S);

        let mut gene = buckle.head;
        loop {
            if gene.is_none() {
                break;
            }

            let Some(cloth) = (self.belt_get)(taker, &gene).onf()? else {
                break;
            };
            let len = (cloth.length as usize).min(cloth.data.len());
            data.extend_from_slice(&cloth.data[..len]);
            gene = cloth.next;
            if cloth.is_end() {
                data.push(0);
                break;
            }
        }

        Ok(data[..].as_utf8_str_null_terminated().to_string())
    }

    pub fn set(
        &self, taker: &Taker, bg: &Gene, data: &str,
    ) -> Result<(), ClientError<E>> {
        bg.validate()?;
        let data = data.as_bytes();

        let mut buckle = (self.buckle_get)(taker, bg)?;
        buckle.length = data.len() as u32;
        (self.buckle_set)(taker, &buckle)?;

        let mut gene = buckle.head;
        let mut cloth = ClothBelt::<S>::default();
        let mut it = data.chunks(S).peekable();

        loop {
            let Some(x) = it.next() else { break };
            cloth.data[x.len()..].fill(0);
            cloth.data[..x.len()].copy_from_slice(x);
            cloth.length = x.len() as u16;
            cloth.set_alive(true);
            cloth.set_is_end(it.peek().is_none());

            if gene.is_none() {
                cloth.gene.clear();
                (self.belt_add)(taker, &buckle.gene, &cloth)?;
                continue;
            }

            cloth.gene = gene;
            match (self.belt_set)(taker, &cloth).onf()? {
                Some(v) => gene = v.next,
                None => {
                    cloth.gene.clear();
                    (self.belt_add)(taker, &buckle.gene, &cloth)?;
                    gene.clear();
                }
            }
        }

        Ok(())
    }
}
