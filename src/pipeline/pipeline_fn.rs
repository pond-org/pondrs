//! The [`PipelineFn`] trait for pipeline factory functions.

use crate::pipeline::Steps;

/// A pipeline factory: given borrowed catalog and params, produces a [`Steps<E>`] pipeline.
///
/// Named functions with tied lifetimes satisfy this automatically via the blanket impl:
///
/// ```ignore
/// fn my_pipeline<'a>(cat: &'a MyCatalog, params: &'a MyParams) -> impl Steps<MyError> + 'a {
///     (Node { .. }, Node { .. })
/// }
/// ```
///
/// The lifetime lives on the trait (not as a GAT), so the blanket impl for `Fn`
/// works without `impl_trait_in_assoc_type`.
pub trait PipelineFn<'a, C: 'a, P: 'a, E> {
    type Output: Steps<E> + 'a;
    fn call(&self, catalog: &'a C, params: &'a P) -> Self::Output;
}

impl<'a, F, C: 'a, P: 'a, E, S> PipelineFn<'a, C, P, E> for F
where
    F: Fn(&'a C, &'a P) -> S,
    S: Steps<E> + 'a,
{
    type Output = S;
    fn call(&self, catalog: &'a C, params: &'a P) -> S {
        (self)(catalog, params)
    }
}
