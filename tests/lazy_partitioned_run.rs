use std::collections::HashMap;

use serde::Serialize;
use tempfile::TempDir;

use pondrs::datasets::{Lazy, LazyDataset, LazyPartitionedDataset, TextDataset};
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::{Node, PartitionedNode, ParallelRunner, Runner};

#[derive(Serialize)]
struct Catalog {
    input: LazyPartitionedDataset<TextDataset>,
    output: LazyPartitionedDataset<TextDataset>,
    output_pnode: LazyPartitionedDataset<TextDataset>,
}

fn copy_texts(
    input: HashMap<String, Lazy<String, PondError>>,
) -> (HashMap<String, Lazy<String, PondError>>,) {
    let output: HashMap<String, Lazy<String, PondError>> = input
        .into_iter()
        .map(|(name, load_thunk)| {
            let save_thunk: Lazy<String, PondError> = Box::new(move || {
                let text = load_thunk()?;
                Ok(text.to_uppercase())
            });
            (name, save_thunk)
        })
        .collect();
    (output,)
}

fn uppercase(text: String) -> (String,) {
    (text.to_uppercase(),)
}

#[test]
fn lazy_partitioned_parallel() {
    let dir = TempDir::new().unwrap();
    let input_dir = dir.path().join("input");
    let output_dir = dir.path().join("output");
    let output_pnode_dir = dir.path().join("output_pnode");
    std::fs::create_dir_all(&input_dir).unwrap();

    let n = 100;
    for i in 0..n {
        let path = input_dir.join(format!("file_{i:03}.txt"));
        std::fs::write(&path, format!("content of file {i:03}")).unwrap();
    }

    let catalog = Catalog {
        input: LazyPartitionedDataset::<TextDataset> {
            path: input_dir.to_str().unwrap().to_string(),
            ext: "txt".into(),
            dataset: LazyDataset {
                dataset: TextDataset::new(""),
            },
        },
        output: LazyPartitionedDataset::<TextDataset> {
            path: output_dir.to_str().unwrap().to_string(),
            ext: "txt".into(),
            dataset: LazyDataset {
                dataset: TextDataset::new(""),
            },
        },
        output_pnode: LazyPartitionedDataset::<TextDataset> {
            path: output_pnode_dir.to_str().unwrap().to_string(),
            ext: "txt".into(),
            dataset: LazyDataset {
                dataset: TextDataset::new(""),
            },
        },
    };

    let pipe = (
        Node {
            name: "copy_texts",
            func: copy_texts,
            input: (&catalog.input,),
            output: (&catalog.output,),
        },
        PartitionedNode {
            name: "uppercase",
            func: uppercase,
            input: &catalog.input,
            output: &catalog.output_pnode,
            _marker: Default::default(),
        },
    );

    let params = ();
    let hooks = (LoggingHook::new(),);
    ParallelRunner::new(5)
        .run::<PondError>(&pipe, &catalog, &params, &hooks)
        .unwrap();

    for i in 0..n {
        let node_path = output_dir.join(format!("file_{i:03}.txt"));
        let pnode_path = output_pnode_dir.join(format!("file_{i:03}.txt"));
        let node_content = std::fs::read_to_string(&node_path).unwrap();
        let pnode_content = std::fs::read_to_string(&pnode_path).unwrap();
        assert_eq!(node_content, format!("CONTENT OF FILE {i:03}"));
        assert_eq!(node_content, pnode_content);
    }
}
