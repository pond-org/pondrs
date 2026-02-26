//! Steps trait and tuple implementations.

use core::marker::Tuple;

use super::traits::PipelineItem;

/// Trait for a sequence of pipeline items (implemented for tuples).
pub trait Steps: Tuple {
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem));
}

impl<N1: PipelineItem> Steps for (N1,) {
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        f(&self.0);
    }
}

impl<N1: PipelineItem, N2: PipelineItem> Steps for (N1, N2) {
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
    }
}

impl<N1: PipelineItem, N2: PipelineItem, N3: PipelineItem> Steps for (N1, N2, N3) {
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
    }
}

impl<N1: PipelineItem, N2: PipelineItem, N3: PipelineItem, N4: PipelineItem> Steps
    for (N1, N2, N3, N4)
{
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
    }
}

impl<N1: PipelineItem, N2: PipelineItem, N3: PipelineItem, N4: PipelineItem, N5: PipelineItem> Steps
    for (N1, N2, N3, N4, N5)
{
    fn for_each_item<'a>(&'a self, f: &mut dyn FnMut(&'a dyn PipelineItem)) {
        f(&self.0);
        f(&self.1);
        f(&self.2);
        f(&self.3);
        f(&self.4);
    }
}
