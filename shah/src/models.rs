#[cfg(feature = "serde")]
use crate::{error::SystemError, Binary};

pub type GeneId = u64;

#[crate::model]
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub struct Gene {
    pub id: GeneId,
    pub iter: u8,
    pub pepper: [u8; 3],
    pub server: u32,
}

impl Gene {
    #[cfg(feature = "serde")]
    pub fn as_hex(&self) -> String {
        let mut dst = [0u8; Gene::S * 2];
        let out = faster_hex::hex_encode(self.as_binary(), &mut dst).unwrap();
        out.to_string()
    }

    pub fn is_none(&self) -> bool {
        self.id == 0
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
}

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
struct StrVisitor;
#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for StrVisitor {
    type Value = String;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a hex string with 32 length")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v.len() != Gene::S * 2 {
            return Err(E::custom(format!(
                "invalid length {}, expected {}",
                v.len(),
                Gene::S * 2
            )));
        }

        Ok(v.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Gene {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer.deserialize_str(StrVisitor)?.parse::<Gene>() {
            Ok(v) => Ok(v),
            Err(_) => Err(serde::de::Error::custom("expected str")),
        }
    }
}

#[crate::model]
#[derive(Debug)]
pub struct OrderHead {
    pub size: u32,
    pub scope: u8,
    pub route: u8,
    _pad: [u8; 2],
    pub id: u64,
}

#[crate::model]
#[derive(Debug)]
pub struct ReplyHead {
    pub id: u64,
    pub size: u32,
    pub error: u32,
    pub elapsed: u64,
}

#[crate::model]
#[derive(Debug)]
pub struct Reply {
    pub head: ReplyHead,
    pub body: [u8; 1024 * 64],
}
