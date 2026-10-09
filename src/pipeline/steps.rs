//! Steps trait and tuple implementations.

use super::check::{CheckError, check_dataset_identity, check_names, check_item, collect_all_outputs, collect_warnings};
#[cfg(feature = "std")]
use super::id_set::{HeapIdSet, HeapIdTypeMap};
use super::id_set::{IdCollector, IdSet, IdTypeMap, TypeCollector};
use super::traits::{StepMeta, Step};
use crate::error::CheckWarning;

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
    /// Under `std` the bookkeeping is heap-allocated and has no capacity
    /// limit. Under `no_std` it uses a fixed capacity of 20 datasets; for
    /// larger pipelines use [`check_with_capacity`](Self::check_with_capacity).
    ///
    /// The error borrows step names from `self`.
    fn check(&self) -> Result<(), CheckError<'_>> {
        #[cfg(feature = "std")]
        {
            run_check::<HeapIdSet, HeapIdTypeMap, _>(self)
        }
        #[cfg(not(feature = "std"))]
        {
            self.check_with_capacity::<20>()
        }
    }

    /// Like [`check`](Self::check), but with stack-allocated bookkeeping of a
    /// fixed dataset capacity `N`. Returns [`CheckError::CapacityExceeded`]
    /// if the pipeline touches more than `N` distinct datasets.
    fn check_with_capacity<const N: usize>(&self) -> Result<(), CheckError<'_>> {
        run_check::<IdSet<N>, IdTypeMap<N>, _>(self)
    }

    /// Report non-fatal diagnostics about dataset identity and naming.
    ///
    /// Unlike [`check`](Self::check), this is not fail-fast: every warning is
    /// handed to `report`. The callback keeps it allocator-free, so it works
    /// under `no_std` — pass a sink that writes to RTT, semihosting or defmt.
    ///
    /// Warnings are never errors. They flag catalogs that run correctly but
    /// whose datasets will be named unreliably in logs, hooks and viz.
    ///
    /// Deduplication is unbounded under `std`; under `no_std` it uses a
    /// capacity of 20 datasets — see
    /// [`for_each_warning_with_capacity`](Self::for_each_warning_with_capacity).
    fn for_each_warning(&self, report: &mut dyn FnMut(&CheckWarning)) {
        #[cfg(feature = "std")]
        {
            let mut seen = HeapIdSet::empty();
            self.for_each_meta(&mut |item| collect_warnings(item, &mut seen, report));
        }
        #[cfg(not(feature = "std"))]
        {
            self.for_each_warning_with_capacity::<20>(report);
        }
    }

    /// Like [`for_each_warning`](Self::for_each_warning), but with a custom
    /// dedup capacity `N`.
    ///
    /// Exceeding `N` is not an error: deduplication simply stops and the same
    /// dataset may be reported more than once.
    fn for_each_warning_with_capacity<const N: usize>(&self, report: &mut dyn FnMut(&CheckWarning)) {
        let mut seen = IdSet::<N>::empty();
        self.for_each_meta(&mut |item| {
            collect_warnings(item, &mut seen, report);
        });
    }
}

/// The three `check` passes, generic over the id bookkeeping.
///
/// A free function rather than a provided method, so the collector types stay
/// out of `StepsMeta`'s public surface.
fn run_check<C: IdCollector, T: TypeCollector, S: StepsMeta + ?Sized>(
    steps: &S,
) -> Result<(), CheckError<'_>> {
    // Names first: a step path must identify one step, which the remaining
    // passes do not depend on but hooks, the cache and CLI filters do.
    check_names(None, &|f| steps.for_each_meta(f))?;

    // Pass 0: dataset identity. Aliased datasets make two distinct datasets
    // look like one, which shows up as spurious `DuplicateOutput` /
    // `InputNotProduced` further down — so report it before those run.
    let mut seen_types = T::empty();
    let mut identity = Ok(());
    steps.for_each_meta(&mut |item| {
        if identity.is_ok() {
            identity = check_dataset_identity(item, &mut seen_types);
        }
    });
    identity?;

    // Pass 1: collect all datasets produced by any node.
    let mut all_produced = C::empty();
    steps.for_each_meta(&mut |item| {
        collect_all_outputs(item, &mut all_produced);
    });

    // Pass 2: walk in order, checking sequential validity.
    let mut produced = C::empty();
    let mut consumed = C::empty();
    let mut result = Ok(());
    steps.for_each_meta(&mut |item| {
        if result.is_ok() {
            result = check_item(item, &all_produced, &mut produced, &mut consumed);
        }
    });
    result
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
