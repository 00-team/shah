use crate::models::Performed;
use crate::ShahError;

pub trait ShahState {
    fn work(&mut self) -> Result<Performed, ShahError>;
}
