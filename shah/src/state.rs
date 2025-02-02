pub trait Task {
    fn work(&mut self);
}

pub trait ShahState<'a> {
    fn tasks(&mut self) -> &'a [impl Task];
}
