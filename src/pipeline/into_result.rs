//! Trait for normalizing node function return types into Result.

use super::node::CompatibleOutput;
use super::stable::StableTuple;

/// Converts a node function's return value into `Result<O, E>`.
///
/// Bare tuples become `Ok(tuple)` (backward compatible).
/// `Result<tuple, E2>` where `E: From<E2>` auto-converts the error.
#[diagnostic::on_unimplemented(
    message = "node function returns `{Self}`, which cannot produce output `{O}` in a pipeline with error type `{E}`",
    label = "invalid node return type",
    note = "return `{O}`, or `Result<{O}, E2>` where `{E}` implements `From<E2>`"
)]
pub trait IntoNodeResult<O: StableTuple, E>: CompatibleOutput<O> {
    fn into_node_result(self) -> Result<O, E>;
}

// Bare tuples -> always Ok
impl<O: StableTuple, E> IntoNodeResult<O, E> for O {
    fn into_node_result(self) -> Result<O, E> {
        Ok(self)
    }
}

// Result<tuple, E2> where E: From<E2> -> convert error
//
// `do_not_recommend`: without it, a missing `From<E2>` is reported as a bare
// unsatisfied `From` bound plus a list of every unrelated `From` impl on the
// pipeline error type. Suppressing this impl as a suggestion makes rustc report
// the `IntoNodeResult` bound itself, so the message above is shown instead.
#[diagnostic::do_not_recommend]
impl<O: StableTuple, E, E2> IntoNodeResult<O, E> for Result<O, E2>
where
    E: From<E2>,
{
    fn into_node_result(self) -> Result<O, E> {
        self.map_err(Into::into)
    }
}
