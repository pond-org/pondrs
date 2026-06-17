use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tempfile::TempDir;

use pondrs::datasets::{MemoryDataset, Param, TextDataset};
use pondrs::error::PondError;
use pondrs::{CacheHook, Dataset, Node, Runner, SequentialRunner, PipelineInfo};

// ---------------------------------------------------------------------------
// Each test group gets its own counter pair to avoid cross-test interference
// (RUST_TEST_THREADS=2).
// ---------------------------------------------------------------------------

macro_rules! define_node_fns {
    ($n1:ident, $n2:ident, $c1:ident, $c2:ident, binary) => {
        static $c1: AtomicUsize = AtomicUsize::new(0);
        static $c2: AtomicUsize = AtomicUsize::new(0);

        fn $n1(input: String, factor: i32) -> (String,) {
            $c1.fetch_add(1, Ordering::SeqCst);
            let val: i32 = input.trim().parse().unwrap();
            ((val * factor).to_string(),)
        }

        fn $n2(input: String) -> (String,) {
            $c2.fetch_add(1, Ordering::SeqCst);
            let val: i32 = input.trim().parse().unwrap();
            ((val + 1).to_string(),)
        }
    };
    ($n1:ident, $n2:ident, $c1:ident, $c2:ident, unary) => {
        static $c1: AtomicUsize = AtomicUsize::new(0);
        static $c2: AtomicUsize = AtomicUsize::new(0);

        fn $n1(input: String) -> (String,) {
            $c1.fetch_add(1, Ordering::SeqCst);
            let val: i32 = input.trim().parse().unwrap();
            ((val * 2).to_string(),)
        }

        fn $n2(input: String) -> (String,) {
            $c2.fetch_add(1, Ordering::SeqCst);
            let val: i32 = input.trim().parse().unwrap();
            ((val + 1).to_string(),)
        }
    };
}

define_node_fns!(file_n1, file_n2, FILE_C1, FILE_C2, binary);
define_node_fns!(param_n1, param_n2, PARAM_C1, PARAM_C2, binary);
define_node_fns!(mem_n1, mem_n2, MEM_C1, MEM_C2, unary);
define_node_fns!(par_n1, par_n2, PAR_C1, PAR_C2, binary);

fn delta(counter: &AtomicUsize, baseline: usize) -> usize {
    counter.load(Ordering::SeqCst) - baseline
}

fn snap(c1: &AtomicUsize, c2: &AtomicUsize) -> (usize, usize) {
    (c1.load(Ordering::SeqCst), c2.load(Ordering::SeqCst))
}

// ---------------------------------------------------------------------------
// Catalog types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct FileCatalog {
    input: TextDataset,
    mid: TextDataset,
    output: TextDataset,
}

#[derive(Serialize)]
struct FileParams {
    factor: Param<i32>,
}

#[derive(Serialize)]
struct MemCatalog {
    input: TextDataset,
    mid: MemoryDataset<String>,
    output: TextDataset,
}

#[derive(Serialize)]
struct EmptyParams;

// ---------------------------------------------------------------------------
// Test: All-file pipeline caching
// ---------------------------------------------------------------------------

#[test]
fn cache_all_file_pipeline() {
    let dir = TempDir::new().unwrap();
    let cache_dir = dir.path().join(".pondcache");

    let input_path = dir.path().join("input.txt");
    std::fs::write(&input_path, "10").unwrap();

    let params = FileParams { factor: Param(2) };
    let catalog = FileCatalog {
        input: TextDataset::new(input_path.to_str().unwrap()),
        mid: TextDataset::new(dir.path().join("mid.txt").to_str().unwrap()),
        output: TextDataset::new(dir.path().join("out.txt").to_str().unwrap()),
    };

    let pipe = (
        Node { name: "node1", func: file_n1, input: (&catalog.input, &params.factor), output: (&catalog.mid,) },
        Node { name: "node2", func: file_n2, input: (&catalog.mid,), output: (&catalog.output,) },
    );
    assert!(pipe.check().is_ok());

    let hooks = (CacheHook::new(&cache_dir),);

    // Run 1: both nodes execute
    let (b1, b2) = snap(&FILE_C1, &FILE_C2);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&FILE_C1, b1), 1);
    assert_eq!(delta(&FILE_C2, b2), 1);
    assert_eq!(catalog.output.load().unwrap(), "21");

    // Run 2: both skip (nothing changed)
    let (b1, b2) = snap(&FILE_C1, &FILE_C2);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&FILE_C1, b1), 0);
    assert_eq!(delta(&FILE_C2, b2), 0);

    // Run 3: modify input file → both re-run
    thread::sleep(Duration::from_millis(50));
    std::fs::write(&input_path, "20").unwrap();
    let hooks = (CacheHook::new(&cache_dir),);
    let (b1, b2) = snap(&FILE_C1, &FILE_C2);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&FILE_C1, b1), 1);
    assert_eq!(delta(&FILE_C2, b2), 1);
    assert_eq!(catalog.output.load().unwrap(), "41");
}

// ---------------------------------------------------------------------------
// Test: Param change invalidates cache
// ---------------------------------------------------------------------------

#[test]
fn cache_invalidates_on_param_change() {
    let dir = TempDir::new().unwrap();
    let cache_dir = dir.path().join(".pondcache");

    let input_path = dir.path().join("input.txt");
    std::fs::write(&input_path, "10").unwrap();

    let params = FileParams { factor: Param(2) };
    let catalog = FileCatalog {
        input: TextDataset::new(input_path.to_str().unwrap()),
        mid: TextDataset::new(dir.path().join("mid.txt").to_str().unwrap()),
        output: TextDataset::new(dir.path().join("out.txt").to_str().unwrap()),
    };

    let pipe = (
        Node { name: "node1", func: param_n1, input: (&catalog.input, &params.factor), output: (&catalog.mid,) },
        Node { name: "node2", func: param_n2, input: (&catalog.mid,), output: (&catalog.output,) },
    );

    let hooks = (CacheHook::new(&cache_dir),);
    let (b1, b2) = snap(&PARAM_C1, &PARAM_C2);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&PARAM_C1, b1), 1);
    assert_eq!(delta(&PARAM_C2, b2), 1);

    // Change param → rebuild with new param
    let params2 = FileParams { factor: Param(3) };
    let catalog2 = FileCatalog {
        input: TextDataset::new(input_path.to_str().unwrap()),
        mid: TextDataset::new(dir.path().join("mid.txt").to_str().unwrap()),
        output: TextDataset::new(dir.path().join("out.txt").to_str().unwrap()),
    };
    let pipe2 = (
        Node { name: "node1", func: param_n1, input: (&catalog2.input, &params2.factor), output: (&catalog2.mid,) },
        Node { name: "node2", func: param_n2, input: (&catalog2.mid,), output: (&catalog2.output,) },
    );
    let hooks2 = (CacheHook::new(&cache_dir),);
    let (b1, b2) = snap(&PARAM_C1, &PARAM_C2);
    SequentialRunner.run::<PondError>(&pipe2, &catalog2, &params2, &hooks2).unwrap();
    assert_eq!(delta(&PARAM_C1, b1), 1);
    assert_eq!(delta(&PARAM_C2, b2), 1);
    assert_eq!(catalog2.output.load().unwrap(), "31");
}

// ---------------------------------------------------------------------------
// Test: Memory intermediate pipeline caching
// ---------------------------------------------------------------------------

#[test]
fn cache_memory_intermediate() {
    let dir = TempDir::new().unwrap();
    let cache_dir = dir.path().join(".pondcache");

    let input_path = dir.path().join("input.txt");
    std::fs::write(&input_path, "10").unwrap();

    let catalog = MemCatalog {
        input: TextDataset::new(input_path.to_str().unwrap()),
        mid: MemoryDataset::new(),
        output: TextDataset::new(dir.path().join("out.txt").to_str().unwrap()),
    };

    let pipe = (
        Node { name: "node1", func: mem_n1, input: (&catalog.input,), output: (&catalog.mid,) },
        Node { name: "node2", func: mem_n2, input: (&catalog.mid,), output: (&catalog.output,) },
    );
    assert!(pipe.check().is_ok());

    let params = EmptyParams;
    let hooks = (CacheHook::new(&cache_dir),);

    // Run 1: both execute
    let (b1, b2) = snap(&MEM_C1, &MEM_C2);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&MEM_C1, b1), 1);
    assert_eq!(delta(&MEM_C2, b2), 1);
    assert_eq!(catalog.output.load().unwrap(), "21");

    // Run 2: node1 re-runs (memory output not persistent), node2 skips
    let hooks = (CacheHook::new(&cache_dir),);
    let (b1, b2) = snap(&MEM_C1, &MEM_C2);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&MEM_C1, b1), 1);
    assert_eq!(delta(&MEM_C2, b2), 0);

    // Run 3: modify input → both re-run
    thread::sleep(Duration::from_millis(50));
    std::fs::write(&input_path, "20").unwrap();
    let hooks = (CacheHook::new(&cache_dir),);
    let (b1, b2) = snap(&MEM_C1, &MEM_C2);
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&MEM_C1, b1), 1);
    assert_eq!(delta(&MEM_C2, b2), 1);
    assert_eq!(catalog.output.load().unwrap(), "41");
}

// ---------------------------------------------------------------------------
// Test: Parallel runner — no deadlocks on skip
// ---------------------------------------------------------------------------

#[test]
fn cache_parallel_no_deadlock() {
    let dir = TempDir::new().unwrap();
    let cache_dir = dir.path().join(".pondcache");

    let input_path = dir.path().join("input.txt");
    std::fs::write(&input_path, "10").unwrap();

    let params = FileParams { factor: Param(2) };
    let catalog = FileCatalog {
        input: TextDataset::new(input_path.to_str().unwrap()),
        mid: TextDataset::new(dir.path().join("mid.txt").to_str().unwrap()),
        output: TextDataset::new(dir.path().join("out.txt").to_str().unwrap()),
    };

    let pipe = (
        Node { name: "node1", func: par_n1, input: (&catalog.input, &params.factor), output: (&catalog.mid,) },
        Node { name: "node2", func: par_n2, input: (&catalog.mid,), output: (&catalog.output,) },
    );

    let hooks = (CacheHook::new(&cache_dir),);

    // First run with parallel
    let (b1, b2) = snap(&PAR_C1, &PAR_C2);
    pondrs::ParallelRunner::new(2).run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&PAR_C1, b1), 1);
    assert_eq!(delta(&PAR_C2, b2), 1);

    // Second run: both should skip without deadlock
    let hooks = (CacheHook::new(&cache_dir),);
    let (b1, b2) = snap(&PAR_C1, &PAR_C2);
    pondrs::ParallelRunner::new(2).run::<PondError>(&pipe, &catalog, &params, &hooks).unwrap();
    assert_eq!(delta(&PAR_C1, b1), 0);
    assert_eq!(delta(&PAR_C2, b2), 0);
}
