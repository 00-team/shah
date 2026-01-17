use crate::models::GeneId;

#[crate::model]
pub struct ShahProgress {
    pub total: GeneId,
    pub prog: GeneId,
}

impl std::fmt::Debug for ShahProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.prog.0, self.total.0)
    }
}

impl ShahProgress {
    pub fn end(&mut self) {
        self.prog = self.total;
    }

    pub fn ended(&self) -> bool {
        self.prog == self.total
    }
}

impl Iterator for ShahProgress {
    type Item = GeneId;
    fn next(&mut self) -> Option<Self::Item> {
        if self.prog >= self.total {
            return None;
        }

        if self.prog == 0 {
            self.prog += 1;
        }

        let id = self.prog;
        self.prog += 1;
        Some(id)
    }
}
