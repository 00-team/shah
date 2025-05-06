use std::{
    fmt::Display,
    ops::{Add, AddAssign, Mul, SubAssign},
};

use super::{Binary, Schema, ShahSchema};
use crate::error::{NotFound, ShahError, SystemError};

#[derive(Default, Debug, PartialEq, PartialOrd, Ord, Clone, Copy, Hash, Eq)]
pub struct GeneId(pub u64);

impl Binary for GeneId {}

impl Display for GeneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl PartialEq<u64> for GeneId {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl Mul for GeneId {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_mul(rhs.0))
    }
}

impl Mul<u64> for GeneId {
    type Output = Self;
    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_mul(rhs))
    }
}

impl AddAssign for GeneId {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl AddAssign<u64> for GeneId {
    fn add_assign(&mut self, rhs: u64) {
        self.0 = self.0.saturating_add(rhs);
    }
}

impl Add for GeneId {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Add<u64> for GeneId {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl SubAssign<u64> for GeneId {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 = self.0.saturating_sub(rhs)
    }
}

impl ShahSchema for GeneId {
    fn shah_schema() -> Schema {
        Schema::U64
    }
}

#[crate::model]
#[derive(Debug, PartialEq, Hash, Eq)]
pub struct Gene {
    pub id: GeneId,
    pub iter: u8,
    pub pepper: [u8; 3],
    pub server: u32,
}

impl ShahSchema for Gene {
    fn shah_schema() -> Schema {
        Schema::Gene
    }
}

impl Gene {
    pub const NONE: Self =
        Self { id: GeneId(0), iter: 0, server: 0, pepper: [0, 0, 0] };

    #[cfg(feature = "serde")]
    pub fn as_hex(&self) -> String {
        use super::Binary;

        let mut dst = [0u8; Gene::S * 2];
        let out = faster_hex::hex_encode(self.as_binary(), &mut dst).unwrap();
        out.to_string()
    }

    pub fn clear(&mut self) {
        self.id = GeneId(0);
        self.iter = 0;
        self.pepper = [0u8; 3];
        self.server = 0;
    }

    pub fn is_none(&self) -> bool {
        self.id.0 == 0
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn exhausted(&self) -> bool {
        self.iter >= 250
    }

    pub fn validate(&self) -> Result<(), ShahError> {
        if self.id.0 == 0 {
            return Err(NotFound::GeneIdZero)?;
        }
        // TODO: get the correct server value
        // if !cfg!(debug_assertions) && self.server == 0 {
        //     return Err(NotFound::GeneServerZero)?;
        // }

        Ok(())
    }

    pub fn check(&self, other: &Self, ls: &str) -> Result<(), ShahError> {
        if self.id != other.id {
            log::error!("{ls} gene id {:?} != {:?}", self.id, other.id);
            return Err(SystemError::GeneIdMismatch)?;
        }

        if self.iter != other.iter {
            log::warn!("{ls} gene iter {} != {}", self.iter, other.iter);
            return Err(NotFound::GeneIterMismatch)?;
        }

        if self.pepper != other.pepper {
            log::warn!(
                "{ls} gene pepper {:?} != {:?}",
                self.pepper,
                other.pepper
            );
            return Err(NotFound::GenePepperMismatch)?;
        }

        Ok(())
    }
}

#[cfg(feature = "serde")]
impl utoipa::PartialSchema for Gene {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi as u;
        u::ObjectBuilder::new()
            .schema_type(u::schema::SchemaType::Type(u::Type::String))
            .title(Some("Gene"))
            .description(Some("universally unique gene if the item"))
            // .examples(["00000000000000000000000000000000"])
            .max_length(Some(32))
            .min_length(Some(32))
            // .pattern(Some("^([a-f0-9]|[A-F0-9]){32}$"))
            .into()
    }
}

#[cfg(feature = "serde")]
impl utoipa::ToSchema for Gene {}

#[cfg(feature = "serde")]
impl std::str::FromStr for Gene {
    type Err = SystemError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut gene = Gene::default();
        let src = s.as_bytes();
        if let Err(e) = faster_hex::hex_decode(src, gene.as_binary_mut()) {
            log::warn!("hex error: {e:?}");
            return Err(SystemError::GeneFromHexErr);
        }
        Ok(gene)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Gene {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.is_none() {
            serializer.serialize_none()
        } else {
            serializer.serialize_str(&self.as_hex())
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Gene {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let Some(v) = Option::<String>::deserialize(de)? else {
            return Ok(Gene::default());
        };

        if v.len() != Gene::S * 2 {
            return Err(serde::de::Error::custom(format!(
                "invalid length {}, expected {}",
                v.len(),
                Gene::S * 2
            )));
        }

        match v.parse::<Gene>() {
            Ok(v) => Ok(v),
            Err(_) => Err(serde::de::Error::custom("expected string")),
        }
    }
}
