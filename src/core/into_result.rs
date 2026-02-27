//! Trait for normalizing node function return types into Result.

use core::marker::Tuple;

/// Converts a node function's return value into `Result<O, E>`.
///
/// Bare tuples become `Ok(tuple)` (backward compatible).
/// `Result<tuple, E2>` where `E: From<E2>` auto-converts the error.
pub trait IntoNodeResult<O: Tuple, E> {
    fn into_node_result(self) -> Result<O, E>;
}

// Bare tuples -> always Ok
impl<O: Tuple, E> IntoNodeResult<O, E> for O {
    fn into_node_result(self) -> Result<O, E> {
        Ok(self)
    }
}

// Result<tuple, E2> where E: From<E2> -> convert error
impl<O: Tuple, E, E2> IntoNodeResult<O, E> for Result<O, E2>
where
    E: From<E2>,
{
    fn into_node_result(self) -> Result<O, E> {
        self.map_err(Into::into)
    }
}
