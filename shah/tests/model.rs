#[cfg(test)]
mod tests {

    #[shah::model]
    struct User {
        #[flags(f_1, f_2, f_3, f_4, f_5, f_6, f_7, f8, f9)]
        pub flags: u64,

        #[flags(bits = 3, fb3_1, fb3_2, fb3_3, fb3_4, fb3_5, fb3_6, fb3_7)]
        pub flags_b3: u64,

        #[flags(fa1, fa2, fa3, fa4, fa5, fa6, fa7, fa8, fa9, fa10, fa11)]
        pub flags_arr: [u8; 8],

        #[flags(bits = 3, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12)]
        pub flags_arr_b3: [u8; 8],
    }

    #[test]
    fn test_flags() {
        let mut user = User::default();
        assert_eq!(user.flags, 0);
        assert_eq!(user.flags_b3, 0);
        assert_eq!(user.flags_arr, [0u8; 8]);
        assert_eq!(user.flags_arr_b3, [0u8; 8]);

        user.set_f_1(true);
        user.set_f_2(true);
        user.set_f_3(false);
        user.set_f_4(true);
        user.set_f_5(true);
        user.set_f_6(false);
        user.set_f_7(true);
        assert_eq!(user.flags, 0b1011011);
        assert!(user.f_1());
        assert!(user.f_2());
        assert!(!user.f_3());
        assert!(user.f_4());
        assert!(user.f_5());
        assert!(!user.f_6());
        assert!(user.f_7());

        user.set_fb3_1(0);
        assert_eq!(user.fb3_1(), 0);

        user.set_fb3_2(1);
        assert_eq!(user.fb3_2(), 1);

        user.set_fb3_3(2);
        assert_eq!(user.fb3_3(), 2);

        user.set_fb3_4(4);
        assert_eq!(user.fb3_4(), 4);

        user.set_fb3_6(0b110101);
        assert_eq!(user.fb3_6(), 0b101);

        //                           f6  f5  f4  f3  f2  f1
        assert_eq!(user.flags_b3, 0b_101_000_100_010_001_000);

        user.set_fa1(true);
        user.set_fa2(true);
        // user.set_fa3(true);
        user.set_fa4(true);
        user.set_fa5(true);
        user.set_fa6(true);
        // user.set_fa7(false);
        user.set_fa8(true);
        user.set_fa9(true);
        // user.set_fa10(false);
        user.set_fa11(true);
        assert_eq!(user.flags_arr, [0b10111011, 0b00000101, 0, 0, 0, 0, 0, 0]);
        user.set_fa11(false);
        assert_eq!(user.flags_arr, [0b10111011, 0b00000001, 0, 0, 0, 0, 0, 0]);

        user.set_x1(1); // 3
        assert_eq!(user.x1(), 1);
        user.set_x2(0); // 6
        assert_eq!(user.x2(), 0);
        user.set_x3(1); // 9
        assert_eq!(user.x3(), 1);
        user.set_x4(2); // 12
        assert_eq!(user.x4(), 2);
        user.set_x5(4); // 15
        assert_eq!(user.x5(), 4);
        user.set_x6(5); // 18
        assert_eq!(user.x6(), 5);
        user.set_x7(0b11111101); // 21
        assert_eq!(user.x7(), 0b101);
        user.set_x8(0); // 24
        assert_eq!(user.x8(), 0);
        user.set_x9(1); // 27
        assert_eq!(user.x9(), 1);
        user.set_x10(7); // 30
        assert_eq!(user.x10(), 7);
        user.set_x11(1); // 33
        assert_eq!(user.x11(), 1);
        user.set_x12(4); // 36
        assert_eq!(user.x12(), 4);

        let fa3 = u64::from_le_bytes(user.flags_arr_b3);
        assert_eq!(((fa3 >> 0) & 7) as u8, user.x1());
        assert_eq!(((fa3 >> 3) & 7) as u8, user.x2());
        assert_eq!(((fa3 >> 6) & 7) as u8, user.x3());
        assert_eq!(((fa3 >> 9) & 7) as u8, user.x4());
        assert_eq!(((fa3 >> 12) & 7) as u8, user.x5());
        assert_eq!(((fa3 >> 15) & 7) as u8, user.x6());
        assert_eq!(((fa3 >> 18) & 7) as u8, user.x7());
        assert_eq!(((fa3 >> 21) & 7) as u8, user.x8());
        assert_eq!(((fa3 >> 24) & 7) as u8, user.x9());
        assert_eq!(((fa3 >> 27) & 7) as u8, user.x10());
        assert_eq!(((fa3 >> 30) & 7) as u8, user.x11());
        assert_eq!(((fa3 >> 33) & 7) as u8, user.x12());

        assert_eq!(
            user.flags_arr_b3,
            [
                0b_01_000_001,
                0b_1_100_010_0,
                0b_000_101_10,
                0b_01_111_001,
                0b_0_000_100_0,
                0b_00_000_000,
                0b_00_000_000,
                0b_00_000_000
            ]
        );
    }
}
