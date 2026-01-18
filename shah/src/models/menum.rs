use std::marker::PhantomData;

use crate::models::ShahSchema;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ShahEnum<I: Default + Copy + ShahSchema, E: Copy + From<I> + Into<I>>
{
    inner: I,
    _ph: PhantomData<E>,
}

impl<I: Default + Copy + ShahSchema, E: Copy + From<I> + Into<I>> ShahSchema
    for ShahEnum<I, E>
{
    fn shah_schema() -> super::Schema {
        I::shah_schema()
    }
}

impl<I: Default + Copy + ShahSchema, E: Copy + From<I> + Into<I>> From<E>
    for ShahEnum<I, E>
{
    fn from(value: E) -> Self {
        Self { inner: value.into(), _ph: PhantomData }
    }
}

impl<I: Default + Copy + ShahSchema, E: Copy + From<I> + Into<I>>
    ShahEnum<I, E>
{
    pub fn to_enum(&self) -> E {
        E::from(self.inner)
    }
    pub fn set(&mut self, e: E) {
        self.inner = e.into();
    }
}

// impl<I: Default + Copy, E: Copy + From<I> + Into<I>> utoipa::PartialSchema for ShahEnum<I, E> {
//     fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
//         E::schema()
//     }
// }

#[cfg(feature = "serde")]
impl<
    I: Default + Copy + ShahSchema,
    E: Copy + From<I> + Into<I> + utoipa::PartialSchema,
> utoipa::__dev::ComposeSchema for ShahEnum<I, E>
{
    fn compose(
        _: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
    ) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        <E as utoipa::PartialSchema>::schema()
    }
}

#[cfg(feature = "serde")]
impl<
    I: Default + Copy + ShahSchema,
    E: Copy + From<I> + Into<I> + utoipa::ToSchema,
> utoipa::ToSchema for ShahEnum<I, E>
{
    fn schemas(
        schemas: &mut Vec<(
            String,
            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
        )>,
    ) {
        E::schemas(schemas)
    }
}

#[cfg(feature = "serde")]
impl<
    I: Default + Copy + ShahSchema,
    E: Copy + From<I> + Into<I> + serde::Serialize,
> serde::Serialize for ShahEnum<I, E>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_enum().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<
    'de,
    I: Default + Copy + ShahSchema,
    E: Copy + From<I> + Into<I> + serde::de::Deserialize<'de>,
> serde::Deserialize<'de> for ShahEnum<I, E>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            inner: E::deserialize(deserializer)?.into(),
            _ph: PhantomData,
        })
    }
}
