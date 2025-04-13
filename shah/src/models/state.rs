use super::Worker;

pub trait ShahState<const N: usize>: Worker<N> {}

impl<const N: usize, T: Worker<N>> ShahState<N> for T {}
