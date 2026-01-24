use std::{fmt::Display, marker::PhantomData};

use crate::models::ShahSchema;

#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ShahEnum<I, E> {
    inner: I,
    _ph: PhantomData<E>,
}

impl<I: Copy, E: PartialEq + From<I> + Into<I>> PartialEq<E>
    for ShahEnum<I, E>
{
    fn eq(&self, other: &E) -> bool {
        self.to_enum() == *other
    }
}

impl<I: Display + Copy, E: Copy + From<I> + Into<I> + std::fmt::Debug>
    std::fmt::Debug for ShahEnum<I, E>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:?}", self.inner, self.to_enum())
    }
}

impl<I: ShahSchema, E> ShahSchema for ShahEnum<I, E> {
    fn shah_schema() -> super::Schema {
        I::shah_schema()
    }
}

impl<I, E: Into<I>> From<E> for ShahEnum<I, E> {
    fn from(value: E) -> Self {
        Self { inner: value.into(), _ph: PhantomData }
    }
}

impl<I: Copy, E: From<I> + Into<I>> ShahEnum<I, E> {
    pub fn value(&self) -> I {
        self.inner
    }

    pub fn new(value: I) -> Self {
        let inner = E::from(value).into();
        Self { inner, _ph: PhantomData }
    }

    pub fn to_enum(&self) -> E {
        E::from(self.inner)
    }
    pub fn set(&mut self, e: E) {
        self.inner = e.into();
    }
}

#[cfg(feature = "serde")]
impl<I, E: utoipa::PartialSchema> utoipa::__dev::ComposeSchema
    for ShahEnum<I, E>
{
    fn compose(
        _: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
    ) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        <E as utoipa::PartialSchema>::schema()
    }
}

#[cfg(feature = "serde")]
impl<I, E: utoipa::ToSchema> utoipa::ToSchema for ShahEnum<I, E> {
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
impl<I: Copy, E: Copy + From<I> + Into<I> + serde::Serialize> serde::Serialize
    for ShahEnum<I, E>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_enum().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, I, E: Into<I> + serde::de::Deserialize<'de>> serde::Deserialize<'de>
    for ShahEnum<I, E>
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
