pub trait Binary: Sized + Copy + Send {
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
        unsafe { &*(data.as_ptr() as *const Self) }
    }

    fn from_binary_mut(data: &mut [u8]) -> &mut Self {
        unsafe { &mut *(data.as_mut_ptr() as *mut Self) }
        //let (_, model, _) = unsafe { data.align_to_mut::<Self>() };
        //&mut model[0]
    }

    fn zeroed(&mut self) {
        self.as_binary_mut().fill(0);
    }
}

// pub trait FromBytes {
//     fn from_bytes(data: &[u8]) -> Self;
// }

// impl<T: Sized> Binary for T {}
impl<const N: usize, T: Binary> Binary for [T; N] {}

// impl<T: Binary> Binary for &mut [T] {}
// impl<T: Binary> Binary for &[T] {}

macro_rules! impl_binary {
    ($($ty:ty),*) => {
        $(impl Binary for $ty {})*
    };
}

impl_binary! {
    u8, u16, u32, u64, usize,
    i8, i16, i32, i64, isize,
    f32, f64, bool
}
