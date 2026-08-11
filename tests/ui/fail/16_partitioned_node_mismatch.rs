//! PartitionedNode element function does not match the partition element type.
use pondrs::{PartitionedNode, PondError, RunnableStep};
use pondrs::datasets::{PartitionedDataset, TextDataset};

fn main() {
    let input = PartitionedDataset::<TextDataset> {
        path: "in".into(),
        ext: "txt".into(),
        dataset: TextDataset::new(""),
    };
    let output = PartitionedDataset::<TextDataset> {
        path: "out".into(),
        ext: "txt".into(),
        dataset: TextDataset::new(""),
    };
    let n = PartitionedNode::new("n", |x: i32| (x,), &input, &output);
    let _step: &dyn RunnableStep<PondError> = &n;
}
