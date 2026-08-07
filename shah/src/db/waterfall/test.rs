#[cfg(test)]
mod test {
    use crate::db::waterfall::*;
    use std::{
        collections::{HashMap, HashSet},
        time::Instant,
    };

    #[test]
    fn test() -> Result<(), ShahError> {
        let mut db = WaterfallDb::<u64, Gene, 124>::new("waterfall-test", 12)?;

        let mut check = HashMap::<u64, Gene>::with_capacity(500_000);

        fn rand_u64() -> u64 {
            let mut buf = [0u8; 8];
            crate::utils::getrandom(&mut buf);
            u64::from_le_bytes(buf)
        }

        println!("inserting 500_000");
        let start = Instant::now();
        for _ in 0..200_000 {
            let key = rand_u64();
            let mut pepper = [0u8; 3];
            crate::utils::getrandom(&mut pepper);
            let val = Gene::keyed(rand_u64(), pepper);
            db.insert(key, val)?;
            check.insert(key, val);
        }

        println!("{} insert took: {:?}", check.len(), start.elapsed());
        println!("meta: {:#?}", db.meta);
        let start = Instant::now();

        println!("unique values: {}", check.len());
        assert_eq!(db.meta.count, check.len() as u64);

        for (k, v) in check.iter() {
            assert_eq!(&db.get(k)?, v);
        }
        println!("{} get took: {:?}", check.len(), start.elapsed());

        println!("loading db again");
        let mut db = WaterfallDb::<u64, Gene, 124>::new("waterfall-test", 12)?;

        println!("deleting");
        let start = Instant::now();
        let mut deleted = HashSet::with_capacity(check.len() / 10);
        for (k, v) in check.clone() {
            if rand_u64() % 10 != 7 {
                continue;
            }

            check.remove(&k);
            deleted.insert(k);
            assert_eq!(db.del(&k)?, v);
        }
        println!("{} delete took: {:?}", check.len(), start.elapsed());

        for (k, v) in check.iter() {
            assert_eq!(&db.get(k)?, v);
        }
        for k in deleted {
            assert!(db.get(&k).onf()?.is_none());
        }

        Ok(())
    }
}
