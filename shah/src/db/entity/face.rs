use crate::{
    ShahModel,
    models::{Gene, ShahSchema},
};

// macro_rules! flag {
//     ($name:ident, $set:ident) => {
//         fn $name(&self) -> bool;
//         fn $set(&mut self, $name: bool) -> &mut Self;
//     };
// }

#[cfg_attr(feature = "serde", shah::flags(inner = u8, serde = true))]
#[cfg_attr(not(feature = "serde"), shah::flags(inner = u8, serde = false))]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)
)]
pub struct EntityFlags {
    pub is_alive: bool,
}

pub trait Entity: ShahModel {
    fn gene(&self) -> &Gene;
    fn gene_mut(&mut self) -> &mut Gene;
    fn growth(&self) -> u64;
    fn growth_mut(&mut self) -> &mut u64;
    fn entity_flags(&self) -> &EntityFlags;
    fn entity_flags_mut(&mut self) -> &mut EntityFlags;

    // flag! {is_alive, set_alive}
    // flag! {is_dep_edited, set_dep_edited}
    // flag! {is_dep_private, set_dep_private}
}

pub trait EntityItem: Entity + ShahSchema {}
impl<T: Entity + ShahSchema> EntityItem for T {}
