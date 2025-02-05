use std::fmt::Debug;

use crate::error::ShahError;

pub trait Task: Debug {
    fn work(&mut self) -> Result<bool, ShahError>;
}

pub trait ShahState<'a, 't> {
    fn tasks(&'a mut self) -> Result<Vec<Box<dyn Task + 't>>, ShahError>;
}
