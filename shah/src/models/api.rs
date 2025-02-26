use crate::ErrorCode;

type ApiCaller<State> = fn(
    state: &mut State,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, ErrorCode>;

#[derive(Debug)]
pub struct Api<State> {
    pub name: &'static str,
    pub input_size: usize,
    pub caller: ApiCaller<State>,
}

#[derive(Debug)]
pub struct Scope<State: 'static> {
    pub routes: &'static [Api<State>],
    pub name: &'static str,
    pub scope: usize,
}
