#[cfg(test)]
mod tests {
    const C: u8 = 12;

    #[shah::enum_int(u8)]
    #[derive(Debug, Default, PartialEq, Eq)]
    enum TheEnum {
        #[default]
        A,
        U,
        X,
        K,
        B = 6,
        C = C,
        D,
        E = 14,
        F = 10,
        G,
        H = 40,
    }

    #[test]
    fn test() {
        assert_eq!(TheEnum::C as u8, C);
        assert_eq!(TheEnum::D as u8, C + 1);

        assert_eq!(TheEnum::A, 0.into());
        assert_eq!(TheEnum::U, 1.into());
        assert_eq!(TheEnum::X, 2.into());
        assert_eq!(TheEnum::K, 3.into());
        assert_eq!(TheEnum::B, 6.into());
        assert_eq!(TheEnum::C, C.into());
        assert_eq!(TheEnum::D, (C + 1).into());
        assert_eq!(TheEnum::E, 14.into());
        assert_eq!(TheEnum::F, 10.into());
        assert_eq!(TheEnum::G, 11.into());
        assert_eq!(TheEnum::H, 40.into());
    }
}
