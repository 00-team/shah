#[derive(Debug)]
pub struct DeadList<T, const CAP: usize> {
    len: usize,
    arr: [Option<T>; CAP],
}

impl<T: Clone + PartialEq, const CAP: usize> Default for DeadList<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq, const CAP: usize> DeadList<T, CAP> {
    pub const fn new() -> Self {
        Self { len: 0, arr: [const { None }; CAP] }
    }

    pub fn push(&mut self, value: T) {
        if self.is_full() {
            return;
        }
        let mut empty_slot: Option<usize> = None;
        let mut travel = 0usize;
        for (i, slot) in self.arr.iter().enumerate() {
            if let Some(item) = slot {
                if *item == value {
                    log::warn!("adding a dupe item");
                    return;
                }
                travel += 1;
            }
            if slot.is_none() && empty_slot.is_none() {
                empty_slot = Some(i);
            }
            if empty_slot.is_some() && travel >= self.len {
                break;
            }
        }
        if let Some(idx) = empty_slot {
            self.arr[idx] = Some(value);
            self.len += 1;
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn pop<F: Fn(&T) -> bool>(&mut self, f: F) -> Option<T> {
        if self.is_empty() {
            return None;
        }
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

    pub fn clear(&mut self) {
        self.arr.fill(None);
        self.len = 0;
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
