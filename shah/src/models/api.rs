use crate::ErrorCode;

type ApiCaller<T> = fn(&mut T, &[u8], &mut [u8]) -> Result<usize, ErrorCode>;

#[derive(Debug)]
pub struct Api<T> {
    pub name: &'static str,
    pub input_size: usize,
    pub caller: ApiCaller<T>,
}

#[derive(Debug)]
pub struct Scope<T: 'static> {
    pub routes: &'static [Api<T>],
    pub name: &'static str,
    pub scope: usize,
}
