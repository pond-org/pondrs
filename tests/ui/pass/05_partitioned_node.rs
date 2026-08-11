//! PartitionedNode with a matching element function.
use pondrs::PartitionedNode;
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
    let _n = PartitionedNode::new("n", |s: String| (s.to_uppercase(),), &input, &output);
}
