use std::collections::HashMap;

use serde::Serialize;
use tempfile::TempDir;

use pondrs::datasets::{Lazy, LazyDataset, LazyPartitionedDataset, TextDataset};
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::{Node, ParallelRunner, Runner};

#[derive(Serialize)]
struct Catalog {
    input: LazyPartitionedDataset<TextDataset>,
    output: LazyPartitionedDataset<TextDataset>,
}

fn copy_texts(
    input: HashMap<String, Lazy<String, PondError>>,
) -> (HashMap<String, Lazy<String, PondError>>,) {
    let output: HashMap<String, Lazy<String, PondError>> = input
        .into_iter()
        .map(|(name, load_thunk)| {
            let save_thunk: Lazy<String, PondError> = Box::new(move || {
                // std::thread::sleep(std::time::Duration::from_millis(50));
                let text = load_thunk()?;
                Ok(text.to_uppercase())
            });
            (name, save_thunk)
        })
        .collect();
    (output,)
}

#[test]
fn lazy_partitioned_parallel() {
    let dir = TempDir::new().unwrap();
    let input_dir = dir.path().join("input");
    let output_dir = dir.path().join("output");
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
    };

    let pipe = (Node {
        name: "copy_texts",
        func: copy_texts,
        input: (&catalog.input,),
        output: (&catalog.output,),
    },);

    let params = ();
    let hooks = (LoggingHook::new(),);
    ParallelRunner::new(5)
        .run::<PondError>(&pipe, &catalog, &params, &hooks)
        .unwrap();

    for i in 0..n {
        let path = output_dir.join(format!("file_{i:03}.txt"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, format!("CONTENT OF FILE {i:03}"));
    }
}
