pub type Perm = (usize, u8);
pub trait Perms {
    fn perm_get(&self, perm: Perm) -> bool;
    fn perm_set(&mut self, perm: Perm, value: bool);
}
impl Perms for [u8] {
    fn perm_get(&self, (byte, bit): Perm) -> bool {
        assert!(self.len() > byte);
        let n = self[byte];
        let f = 1 << bit;
        (n & f) == f
    }
    fn perm_set(&mut self, (byte, bit): Perm, value: bool) {
        assert!(self.len() > byte);
        let f = 1 << bit;
        if value {
            self[byte] |= f;
        } else {
            self[byte] &= !f;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Perms;

    crate::perms!(A, _P, _P, _P, _P, _P, _P, _P, _P, C);

    #[test]
    fn perms() {
        let mut perms = [0u8; 3];
        assert_eq!(perms, [0, 0, 0]);

        assert!(!perms.perm_get(A));
        perms.perm_set(A, true);
        assert!(perms.perm_get(A));
        assert_eq!(perms, [1, 0, 0]);
        perms.perm_set(A, false);

        assert!(!perms.perm_get(C));
        perms.perm_set(C, true);
        assert!(perms.perm_get(C));
        assert_eq!(perms, [0, 2, 0]);
    }
}
