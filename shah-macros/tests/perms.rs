#[cfg(test)]
mod tests {
    mod perms {
        shah_macros::perms! {
            A0, A1, A2, A3, A4, A5, A6, A7,
            B0, B1, B2, B3, B4, _P, B6, B7,
            C0, C1, C2, C3, C4, C5, C6, C7,
            D0, D1, D2, D3, D4, D5, D6, D7,
        }
    }

    #[test]
    fn perms() {
        assert_eq!(perms::A0, (0, 0));
        assert_eq!(perms::A7, (0, 7));
        assert_eq!(perms::B0, (1, 0));
        assert_eq!(perms::B7, (1, 7));
    }
}
