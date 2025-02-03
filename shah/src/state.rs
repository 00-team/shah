pub trait Task {
    fn work(&mut self);
}

pub trait ShahState<'a> {
    fn tasks(&'a mut self) -> Vec<impl Task>;
}
