use crate::models::ExampleError;
use crate::models::State;
use shah::db::entity::EntityDb;
use shah::models::{Gene, GeneId};
use shah::{Entity, ShahError, ShahSchema};
use shah::{ErrorCode, PAGE_SIZE};

pub use db::User;

pub(crate) mod db {
    use super::*;

    #[shah::model]
    #[derive(Debug, PartialEq, Clone, Copy, ShahSchema)]
    pub struct SessionInfo {
        pub client: u8,
        pub os: u8,
        pub browser: u8,
        pub device: u8,
        pub client_version: u16,
        pub os_version: u16,
        pub browser_version: u16,
        _pad: [u8; 2],
    }

    #[shah::model]
    #[derive(Debug, PartialEq, Clone, Copy, ShahSchema)]
    pub struct Session {
        ip: [u8; 4],
        info: SessionInfo,
        timestamp: u64,
        token: [u8; 64],
    }

    #[derive(ShahSchema)]
    #[shah::model]
    #[derive(Entity, Debug, PartialEq, Clone, Copy)]
    pub struct User {
        // pub flags: u64,
        pub gene: Gene,
        pub agent: Gene,
        pub review: Gene,
        pub photo: Gene,
        pub reviews: [u64; 3],
        #[str(set = false)]
        phone: [u8; 12],
        pub cc: u16,
        pub entity_flags: u8,

        #[str]
        pub name: [u8; 49],
        pub sessions: [Session; 3],
        growth: u64,

        #[flags(has_dine_in, f_2, f_3, f_4, f_5, f_6, f_7, f8, f9)]
        pub flags: u8,

        #[flags(bits = 3, fb3_1, fb3_2, fb3_3, fb3_4, fb3_5, fb3_6, fb3_7)]
        pub flags_b3: u64,

        #[flags(fa1, fa2, fa3, fa4, fa5, fa6, fa7, fa8, fa9, fa10, fa11)]
        pub flags_arr: [u8; 8],

        #[flags(bits = 3, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12)]
        pub flags_arr_b3: [u8; 8],
    }

    impl User {
        #[allow(dead_code)]
        pub fn set_phone(&mut self, phone: &str) -> Result<(), ExampleError> {
            if phone.len() != 11 || !phone.starts_with("09") {
                return Err(ExampleError::BadPhone);
            }
            if phone.chars().any(|c| !c.is_ascii_digit()) {
                return Err(ExampleError::BadPhone);
            }

            self.phone[..11].clone_from_slice(phone.as_bytes());

            Ok(())
        }
    }

    pub type UserDb = EntityDb<User>;

    #[allow(dead_code)]
    pub(crate) fn init() -> Result<UserDb, ShahError> {
        UserDb::new("user", 1)
    }
}

#[shah::api(scope = 0, error = crate::models::ExampleError)]
mod uapi {

    pub(super) fn user_add(
        state: &mut State, (inp,): (&User,), (out,): (&mut User,),
    ) -> Result<(), ErrorCode> {
        out.clone_from(inp);
        out.gene.id = GeneId(0);
        state.users.add(out)?;
        Ok(())
    }

    pub(crate) fn user_get(
        state: &mut State, (gene,): (&Gene,), (user,): (&mut User,),
    ) -> Result<(), ErrorCode> {
        log::debug!("in user::user_get ");
        log::debug!("inp: {:?}", gene);
        log::debug!("out: {:?}", user);

        state.users.get(gene, user)?;

        log::debug!("out: {:?}", user);

        Ok(())
    }

    pub(crate) fn user_list(
        state: &mut State, (page,): (&GeneId,),
        (users,): (&mut [User; PAGE_SIZE],),
    ) -> Result<usize, ErrorCode> {
        let count = state.users.list(*page, users)?;
        Ok(count * <User as shah::models::Binary>::S)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_flags() {
        let mut user = super::db::User::default();
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
