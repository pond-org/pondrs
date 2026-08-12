//! Steps trait and tuple implementations.

use super::check::{CheckError, check_item, collect_all_outputs};
use super::id_set::IdSet;
use super::traits::{StepMeta, Step};

/// Non-generic trait for a sequence of steps (metadata only).
///
/// The metadata companion to [`Steps`], as [`StepMeta`] is to
/// [`Step`](super::Step). Implemented for tuples of [`StepMeta`] and for
/// `DynSteps`. Provides pipeline validation via [`check`](StepsMeta::check).
pub trait StepsMeta {
    /// Iterate over each step's metadata.
    fn for_each_meta<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta));

    /// Validate sequential ordering and pipeline contracts.
    ///
    /// Checks that no node reads a dataset before it is produced by an
    /// earlier node, that no dataset is produced twice, that params are
    /// not written, and that pipeline declared inputs/outputs match their
    /// children.
    ///
    /// Datasets that are consumed but not produced by any node are treated
    /// as external inputs and are not flagged.
    ///
    /// Uses a default capacity of 20 datasets. For larger pipelines,
    /// use [`check_with_capacity`](Self::check_with_capacity).
    fn check(&self) -> Result<(), CheckError> {
        self.check_with_capacity::<20>()
    }

    /// Like [`check`](Self::check), but with a custom dataset capacity `N`.
    fn check_with_capacity<const N: usize>(&self) -> Result<(), CheckError> {
        // Pass 1: collect all datasets produced by any node.
        let mut all_produced = IdSet::<N>::new();
        self.for_each_meta(&mut |item| {
            collect_all_outputs::<N>(item, &mut all_produced);
        });

        // Pass 2: walk in order, checking sequential validity.
        let mut produced = IdSet::<N>::new();
        let mut consumed = IdSet::<N>::new();
        let mut result = Ok(());
        self.for_each_meta(&mut |item| {
            if result.is_ok() {
                result = check_item::<N>(item, &all_produced, &mut produced, &mut consumed);
            }
        });
        result
    }
}

/// Generic trait for a sequence of executable steps.
///
/// Extends [`StepsMeta`] with the ability to iterate over the steps themselves.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a pipeline",
    label = "not a pipeline",
    note = "a pipeline is a tuple of up to 10 steps, a `Pipeline`, or a `DynSteps`",
    note = "the pipeline error type `{E}` must implement `From<PondError>`"
)]
pub trait Steps<E>: StepsMeta {
    /// Iterate over each executable step.
    fn for_each_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn Step<E>));
}

macro_rules! impl_steps {
    ($($N:ident $idx:tt),+) => {
        impl<$($N: StepMeta),+> StepsMeta for ($($N,)+) {
            fn for_each_meta<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta)) {
                $(f(&self.$idx);)+
            }
        }

        impl<E, $($N: Step<E>),+> Steps<E> for ($($N,)+) {
            fn for_each_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn Step<E>)) {
                $(f(&self.$idx);)+
            }
        }
    };
}

impl_steps!(N0 0);
impl_steps!(N0 0, N1 1);
impl_steps!(N0 0, N1 1, N2 2);
impl_steps!(N0 0, N1 1, N2 2, N3 3);
impl_steps!(N0 0, N1 1, N2 2, N3 3, N4 4);
impl_steps!(N0 0, N1 1, N2 2, N3 3, N4 4, N5 5);
impl_steps!(N0 0, N1 1, N2 2, N3 3, N4 4, N5 5, N6 6);
impl_steps!(N0 0, N1 1, N2 2, N3 3, N4 4, N5 5, N6 6, N7 7);
impl_steps!(N0 0, N1 1, N2 2, N3 3, N4 4, N5 5, N6 6, N7 7, N8 8);
impl_steps!(N0 0, N1 1, N2 2, N3 3, N4 4, N5 5, N6 6, N7 7, N8 8, N9 9);
