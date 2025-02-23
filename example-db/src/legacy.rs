#![allow(dead_code)]

struct Main {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
}

#[shah::legacy]
mod x {
    // use super::Main;
    #[derive(Debug, Default)]
    struct Base {
        a: u64,
        b: u32,
        c: u32,
    }

    impl From<&Main> for Base {
        fn from(value: &Main) -> Self {
            let bbb = value.b as u32;
            Self { b: bbb, c: value.c as u32 }
        }
    }

    struct ChildD {
        d: u16,
    }

    impl From<&Main> for ChildD {
        fn from(value: &Main) -> Self {
            Self { a: 0, d: value.d as u16 }
        }
    }

    struct ChildE {
        e: u16,
    }

    impl From<&Main> for ChildE {
        fn from(value: Main) -> Self {
            Self { a: 10, e: value.e as u16 * 2 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChildD, ChildE, Main};

    #[test]
    fn test_impl_from() {
        let main = Main { a: 1, b: 2, c: 3, d: 4, e: 5 };
        let cd = ChildD::from(&main);
        assert_eq!(cd.a, 0);
        assert_eq!(cd.b, 2);
        assert_eq!(cd.c, 3);
        assert_eq!(cd.d, 4);

        let ce = ChildE::from(main);
        assert_eq!(ce.a, 10);
        assert_eq!(ce.b, 2);
        assert_eq!(ce.c, 3);
        assert_eq!(ce.e, 10);
    }
}
