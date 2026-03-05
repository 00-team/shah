pub struct DeadList<T> {
    vec: Vec<T>,
    disabled: bool,
}

impl<T: Copy + PartialEq + std::fmt::Debug> std::fmt::Debug for DeadList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeadList")
            .field("fist_item", &self.vec[0])
            .field("len", &self.len())
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl<T: Copy + PartialEq> Default for DeadList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + PartialEq> DeadList<T> {
    pub fn new() -> Self {
        Self { vec: Vec::with_capacity(4096), disabled: false }
    }

    pub fn disable(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    pub fn push(&mut self, value: T) {
        if self.disabled {
            log::warn!("pushing on a disabled DeadList");
            return;
        }

        self.vec.push(value);
    }

    pub const fn len(&self) -> usize {
        self.vec.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.disabled {
            log::warn!("poping on a disabled DeadList");
            return None;
        }
        if self.is_empty() {
            return None;
        }
        self.vec.pop()
    }

    pub fn clear(&mut self) {
        if self.disabled {
            log::warn!("clearing a disabled DeadList");
            return;
        }
        self.vec.clear();
    }
}
