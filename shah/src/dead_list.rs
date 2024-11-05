use core::panic;

#[derive(Debug)]
pub struct DeadList<T, const CAP: usize> {
    len: usize,
    arr: [Option<T>; CAP],
}

impl<T: Clone, const CAP: usize> DeadList<T, CAP> {
    pub const fn new() -> Self {
        Self { len: 0, arr: [const { None }; CAP] }
    }

    pub fn push(&mut self, item: T) {
        if self.is_full() {
            return;
        }
        for slot in self.arr.iter_mut() {
            if slot.is_none() {
                *slot = Some(item);
                self.len += 1;
                break;
            }
        }
    }

    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn pop<F: Fn(&T) -> bool>(&mut self, f: F) -> Option<T> {
        let mut travel = 0usize;
        for slot in self.arr.iter_mut() {
            if let Some(item) = slot {
                if f(item) {
                    let v = item.clone();
                    *slot = None;
                    self.len -= 1;
                    return Some(v);
                }
                travel += 1;
            }

            if travel >= self.len {
                break;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn arrvec() {
        let mut dl = super::DeadList::<u8, 10>::new();
        assert_eq!(dl.len(), 0);
        dl.push(12);
        assert_eq!(dl.len(), 1);
        assert_eq!(dl.pop(|v| *v != 0), Some(12));
        assert_eq!(dl.len(), 0);
        assert_eq!(dl.pop(|v| *v != 0), None);
    }
}
