use super::BeltDb;
use crate::db::entity::{Entity, EntityFlags};
use crate::models::ShahString;
use crate::{AsUtf8Str, IsNotFound, OptNotFound, ShahError};
use crate::{ClientError, Taker, models::Gene};

#[cfg_attr(feature = "serde", shah::flags(inner = u8, serde = true))]
#[cfg_attr(not(feature = "serde"), shah::flags(inner = u8, serde = false))]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)
)]
pub struct ClothFlags {
    is_end: bool,
}

#[derive(shah::ShahSchema)]
#[shah::model]
#[derive(Debug, shah::Entity, shah::Belt)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, utoipa::ToSchema))]
pub struct ClothBelt<const S: usize> {
    pub gene: Gene,
    pub next: Gene,
    pub past: Gene,
    pub buckle: Gene,
    growth: u64,
    pub length: u16,
    entity_flags: EntityFlags,
    flags: ClothFlags,
    #[cfg_attr(feature = "serde", serde(skip))]
    _pad: [u8; 4],
    #[schema(value_type = String)]
    pub data: ShahString<S>,
}

#[derive(shah::ShahSchema)]
#[shah::model]
#[derive(Debug, shah::Entity, shah::Buckle)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, utoipa::ToSchema))]
pub struct ClothBuckle {
    pub gene: Gene,
    pub head: Gene,
    pub tail: Gene,
    #[buckle(belt_count)]
    pub chunks: u64,
    pub owner: Gene,
    pub growth: u64,
    entity_flags: EntityFlags,
    #[cfg_attr(feature = "serde", serde(skip))]
    _pad: [u8; 3],
    pub length: u32,
}

pub type BeltClothDb<const S: usize> = BeltDb<ClothBelt<S>, ClothBuckle>;

impl<const S: usize> BeltClothDb<S> {
    pub fn get(&mut self, bg: &Gene) -> Result<String, ShahError> {
        let mut buckle = ClothBuckle::default();
        self.buckle_get(bg, &mut buckle)?;

        let mut data = Vec::with_capacity(buckle.chunks as usize * S + 10);

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
            data.extend_from_slice(&cloth.data.raw()[..len]);
            gene = cloth.next;
            if cloth.flags.is_end() {
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

        while let Some(x) = it.next() {
            cloth.data.raw_mut()[x.len()..].fill(0);
            cloth.data.raw_mut()[..x.len()].copy_from_slice(x);
            cloth.length = x.len() as u16;
            cloth.entity_flags_mut().set_is_alive(true);
            cloth.flags.set_is_end(it.peek().is_none());

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
        buckle_gene.validate()?;
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
            data.extend_from_slice(&cloth.data.raw()[..len]);
            gene = cloth.next;
            if cloth.flags.is_end() {
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

        while let Some(x) = it.next() {
            cloth.data.raw_mut()[x.len()..].fill(0);
            cloth.data.raw_mut()[..x.len()].copy_from_slice(x);
            cloth.length = x.len() as u16;
            cloth.entity_flags_mut().set_is_alive(true);
            cloth.flags.set_is_end(it.peek().is_none());

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
