//! Steps trait and tuple implementations.

use core::marker::Tuple;

use super::check::{CheckError, check_item, collect_all_outputs};
use super::id_set::IdSet;
use super::traits::{PipelineInfo, PipelineItem};

/// Non-generic trait for a sequence of pipeline items (metadata only).
pub trait StepInfo: Tuple {
    fn for_each_info<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo));

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
        self.for_each_info(&mut |item| {
            collect_all_outputs::<N>(item, &mut all_produced);
        });

        // Pass 2: walk in order, checking sequential validity.
        let mut produced = IdSet::<N>::new();
        let mut consumed = IdSet::<N>::new();
        let mut result = Ok(());
        self.for_each_info(&mut |item| {
            if result.is_ok() {
                result = check_item::<N>(item, &all_produced, &mut produced, &mut consumed);
            }
        });
        result
    }
}

/// Generic trait for a sequence of executable pipeline items.
pub trait Steps<E>: StepInfo {
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>));
}

// --- 1-element tuple ---

impl<N1: PipelineInfo> StepInfo for (N1,) {
    fn for_each_info<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
        f(&self.0);
    }
}

impl<E, N1: PipelineItem<E>> Steps<E> for (N1,) {
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>)) {
        f(&self.0);
    }
}

// --- 2-element tuple ---

impl<N1: PipelineInfo, N2: PipelineInfo> StepInfo for (N1, N2) {
    fn for_each_info<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
        f(&self.0);
        f(&self.1);
    }
}

impl<E, N1: PipelineItem<E>, N2: PipelineItem<E>> Steps<E> for (N1, N2) {
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>)) {
        f(&self.0);
        f(&self.1);
    }
}

// --- 3-element tuple ---

impl<N1: PipelineInfo, N2: PipelineInfo, N3: PipelineInfo> StepInfo for (N1, N2, N3) {
    fn for_each_info<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
    }
}

impl<E, N1: PipelineItem<E>, N2: PipelineItem<E>, N3: PipelineItem<E>> Steps<E>
    for (N1, N2, N3)
{
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
    }
}

// --- 4-element tuple ---

impl<N1: PipelineInfo, N2: PipelineInfo, N3: PipelineInfo, N4: PipelineInfo> StepInfo
    for (N1, N2, N3, N4)
{
    fn for_each_info<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
    }
}

impl<E, N1: PipelineItem<E>, N2: PipelineItem<E>, N3: PipelineItem<E>, N4: PipelineItem<E>>
    Steps<E> for (N1, N2, N3, N4)
{
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
    }
}

// --- 5-element tuple ---

impl<
    N1: PipelineInfo,
    N2: PipelineInfo,
    N3: PipelineInfo,
    N4: PipelineInfo,
    N5: PipelineInfo,
> StepInfo for (N1, N2, N3, N4, N5)
{
    fn for_each_info<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
        f(&self.4);
    }
}

impl<
    E,
    N1: PipelineItem<E>,
    N2: PipelineItem<E>,
    N3: PipelineItem<E>,
    N4: PipelineItem<E>,
    N5: PipelineItem<E>,
> Steps<E> for (N1, N2, N3, N4, N5)
{
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem<E>)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
        f(&self.4);
    }
}
