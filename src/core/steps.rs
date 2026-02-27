//! Steps trait and tuple implementations.

use core::marker::Tuple;

use super::traits::{PipelineInfo, PipelineItem};

/// Non-generic trait for a sequence of pipeline items (metadata only).
pub trait StepInfo: Tuple {
    fn for_each_info<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineInfo));
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
