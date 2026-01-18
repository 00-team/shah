#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShahString<const N: usize> {
    inner: [u8; N],
}

impl<const N: usize> ShahString<N> {
    pub fn as_str(&self) -> &str {
        shah::AsUtf8Str::as_utf8_str_null_terminated(&self.inner)
    }

    pub fn set(&mut self, value: &str) -> bool {
        if value.is_empty() {
            self.inner.fill(0);
            return false;
        }

        let mut overflow = false;
        let vlen = value.len();
        let len = if vlen > N {
            overflow = true;
            let mut idx = N;
            loop {
                if value.is_char_boundary(idx) {
                    break idx;
                }
                idx -= 1;
                continue;
            }
        } else {
            vlen
        };

        self.inner[..len].copy_from_slice(&value.as_bytes()[..len]);
        if len < N {
            self.inner[len] = 0;
        }

        overflow
    }
}

impl<const N: usize> From<String> for ShahString<N> {
    fn from(value: String) -> Self {
        let mut ss = Self::default();
        ss.set(&value);
        ss
    }
}

impl<const N: usize> From<&str> for ShahString<N> {
    fn from(value: &str) -> Self {
        let mut ss = Self::default();
        ss.set(value);
        ss
    }
}

impl<const N: usize> Default for ShahString<N> {
    fn default() -> Self {
        Self { inner: [0; N] }
    }
}

impl<const N: usize> std::fmt::Display for ShahString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<const N: usize> utoipa::__dev::ComposeSchema for ShahString<N> {
    fn compose(
        _: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
    ) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        <String as utoipa::PartialSchema>::schema()
    }
}

#[cfg(feature = "serde")]
impl<const N: usize> utoipa::ToSchema for ShahString<N> {
    fn schemas(
        schemas: &mut Vec<(
            String,
            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
        )>,
    ) {
        String::schemas(schemas)
    }
}

#[cfg(feature = "serde")]
impl<const N: usize> serde::Serialize for ShahString<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, const N: usize> serde::Deserialize<'de> for ShahString<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = String::deserialize(deserializer)?;
        Ok(Self::from(str))
    }
}

impl<const N: usize> super::ShahSchema for ShahString<N> {
    fn shah_schema() -> super::Schema {
        super::Schema::Array {
            is_str: true,
            length: N as u64,
            kind: Box::new(super::Schema::U8),
        }
    }
}
