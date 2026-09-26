use std::collections::BTreeMap;
use std::marker::PhantomData;

use serde::Serialize;
use tempfile::TempDir;

use pondrs::datasets::{
    Dataset, Lazy, LazyDataset, LazyPartitionedDataset, Loader, PartitionedDataset, TextDataset,
};
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
    input: BTreeMap<String, Loader<String, PondError>>,
) -> (BTreeMap<String, Lazy<String, PondError>>,) {
    let output: BTreeMap<String, Lazy<String, PondError>> = input
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
            _marker: PhantomData,
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

fn lazy_text(dir: &std::path::Path) -> LazyPartitionedDataset<TextDataset> {
    LazyPartitionedDataset::<TextDataset> {
        path: dir.to_str().unwrap().to_string(),
        ext: "txt".into(),
        dataset: LazyDataset {
            dataset: TextDataset::new(""),
        },
    }
}

fn eager_text(dir: &std::path::Path) -> PartitionedDataset<TextDataset> {
    PartitionedDataset {
        path: dir.to_str().unwrap().to_string(),
        ext: "txt".into(),
        dataset: TextDataset::new(""),
    }
}

fn write_inputs(dir: &std::path::Path, names: &[&str]) {
    std::fs::create_dir_all(dir).unwrap();
    for name in names {
        std::fs::write(dir.join(format!("{name}.txt")), format!("content of {name}")).unwrap();
    }
}

#[test]
fn lazy_partitioned_loaders_are_ordered_and_repeatable() {
    let dir = TempDir::new().unwrap();
    // Written out of order, so a sorted result can't come from creation order.
    let names = ["delta", "alpha", "charlie", "bravo", "echo"];
    write_inputs(dir.path(), &names);

    let loaders = lazy_text(dir.path()).load().unwrap();

    let keys: Vec<&str> = loaders.keys().map(String::as_str).collect();
    assert_eq!(keys, ["alpha", "bravo", "charlie", "delta", "echo"]);

    for (name, loader) in &loaders {
        let first = loader().unwrap();
        let second = loader().unwrap();
        assert_eq!(first, format!("content of {name}"));
        assert_eq!(first, second);
    }

    // Each call re-reads the file: nothing is cached.
    let loader = &loaders["alpha"];
    std::fs::write(dir.path().join("alpha.txt"), "rewritten").unwrap();
    assert_eq!(loader().unwrap(), "rewritten");

    let loader = loaders["bravo"].clone();
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let loader = loader.clone();
                s.spawn(move || loader().unwrap())
            })
            .collect();
        for h in handles {
            assert_eq!(h.join().unwrap(), "content of bravo");
        }
    });
}

#[derive(Serialize)]
struct LazyToEagerCatalog {
    input: LazyPartitionedDataset<TextDataset>,
    output: PartitionedDataset<TextDataset>,
}

#[test]
fn partitioned_node_lazy_to_eager() {
    let dir = TempDir::new().unwrap();
    let names = ["a", "b", "c"];
    write_inputs(&dir.path().join("input"), &names);

    let catalog = LazyToEagerCatalog {
        input: lazy_text(&dir.path().join("input")),
        output: eager_text(&dir.path().join("output")),
    };
    let pipe = (PartitionedNode {
        name: "uppercase",
        func: uppercase,
        input: &catalog.input,
        output: &catalog.output,
        _marker: PhantomData,
    },);

    ParallelRunner::new(2)
        .run::<PondError>(&pipe, &catalog, &(), &())
        .unwrap();

    for name in names {
        let content = std::fs::read_to_string(dir.path().join(format!("output/{name}.txt"))).unwrap();
        assert_eq!(content, format!("CONTENT OF {}", name.to_uppercase()));
    }
}

#[derive(Serialize)]
struct EagerToLazyCatalog {
    input: PartitionedDataset<TextDataset>,
    output: LazyPartitionedDataset<TextDataset>,
}

#[test]
fn partitioned_node_eager_to_lazy() {
    let dir = TempDir::new().unwrap();
    let names = ["a", "b", "c"];
    write_inputs(&dir.path().join("input"), &names);

    let catalog = EagerToLazyCatalog {
        input: eager_text(&dir.path().join("input")),
        output: lazy_text(&dir.path().join("output")),
    };
    let pipe = (PartitionedNode {
        name: "uppercase",
        func: uppercase,
        input: &catalog.input,
        output: &catalog.output,
        _marker: PhantomData,
    },);

    ParallelRunner::new(2)
        .run::<PondError>(&pipe, &catalog, &(), &())
        .unwrap();

    for name in names {
        let content = std::fs::read_to_string(dir.path().join(format!("output/{name}.txt"))).unwrap();
        assert_eq!(content, format!("CONTENT OF {}", name.to_uppercase()));
    }
}
