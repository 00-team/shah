pub trait Binary: Sized {
    const S: usize = core::mem::size_of::<Self>();
    const N: u64 = core::mem::size_of::<Self>() as u64;

    fn as_binary(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }
    fn as_binary_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                self as *mut Self as *mut u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    fn from_binary(data: &[u8]) -> &Self {
        let (_, model, _) = unsafe { data.align_to::<Self>() };
        &model[0]
    }
    fn from_binary_mut(data: &mut [u8]) -> &mut Self {
        let (_, model, _) = unsafe { data.align_to_mut::<Self>() };
        &mut model[0]
    }
}

pub trait FromBytes {
    fn from_bytes(data: &[u8]) -> Self;
}

impl<T: Sized> Binary for T {}
