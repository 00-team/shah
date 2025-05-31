use crate::ErrorCode;

type ApiCaller<State> = fn(
    state: &mut State,
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, ErrorCode>;

#[derive(Debug)]
pub struct Api<State> {
    pub name: &'static str,
    pub caller: ApiCaller<State>,
    pub input_size: usize,
    pub max_output_size: usize,
}

#[derive(Debug)]
pub struct Scope<State: 'static> {
    pub routes: &'static [Api<State>],
    pub name: &'static str,
    pub scope: usize,
}
