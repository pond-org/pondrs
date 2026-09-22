#![allow(dead_code)]

//! End-to-end tests for `RecurrentNode`, via the recurrent example pipeline.

#[path = "../examples/recurrent/mod.rs"]
mod recurrent;

use std::path::Path;
use std::sync::{Arc, Mutex};

use pondrs::app::App;
use pondrs::datasets::{JsonDataset, Param, TextDataset};
use pondrs::error::PondError;
use pondrs::{CacheHook, Hook, HookAbort, StepMeta};
use recurrent::{pipeline, write_fixtures, Catalog, Params};

fn args(dir: &Path, extra: &[&str]) -> Vec<String> {
    let mut v = vec![
        "test".to_string(),
        "--catalog-path".into(),
        dir.join("catalog.yml").to_str().unwrap().into(),
        "--params-path".into(),
        dir.join("params.yml").to_str().unwrap().into(),
    ];
    v.extend(extra.iter().map(|s| (*s).to_string()));
    v
}

fn read(dir: &Path, k: usize) -> f64 {
    std::fs::read_to_string(dir.join(format!("ckpt/epoch_{k}.txt")))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

/// Records the name of every node that actually ran (not skipped).
#[derive(Clone, Default)]
struct RanNodes(Arc<Mutex<Vec<String>>>);

impl Hook for RanNodes {
    fn after_node_run(&self, n: &dyn StepMeta, skipped: bool) -> Result<(), HookAbort> {
        if !skipped {
            self.0.lock().unwrap().push(n.name().to_string());
        }
        Ok(())
    }
}

#[test]
fn check_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    App::from_args(args(dir.path(), &["check"])).unwrap().dispatch(pipeline).unwrap();
}

#[test]
fn run_chains_each_iteration_through_its_own_slot() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    App::from_args(args(dir.path(), &["run"])).unwrap().dispatch(pipeline).unwrap();

    // w_{k+1} = w_k - 0.25 * 2 (w_k - 10) = (w_k + 10) / 2, from w = 0.
    let expected = [5.0, 7.5, 8.75, 9.375, 9.6875];
    for (k, want) in expected.iter().enumerate() {
        assert!((read(dir.path(), k) - want).abs() < 1e-12, "slot {k}");
    }
    let report = std::fs::read_to_string(dir.path().join("report.json")).unwrap();
    assert!(report.contains("9.6875"), "{report}");
}

#[test]
fn catalog_override_changes_the_iteration_count() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    App::from_args(args(dir.path(), &["run", "--catalog", "checkpoints.count=2"]))
        .unwrap()
        .dispatch(pipeline)
        .unwrap();
    assert!(dir.path().join("ckpt/epoch_1.txt").exists());
    assert!(!dir.path().join("ckpt/epoch_2.txt").exists());
}

#[test]
fn from_nodes_resumes_mid_chain() {
    let dir = tempfile::tempdir().unwrap();
    write_fixtures(dir.path());
    App::from_args(args(dir.path(), &["run"])).unwrap().dispatch(pipeline).unwrap();

    // Plant a different slot 0; resuming from iteration 1 must read it and
    // must not rewrite it.
    std::fs::write(dir.path().join("ckpt/epoch_0.txt"), "2").unwrap();
    App::from_args(args(dir.path(), &["run", "--from-nodes", "train/1"]))
        .unwrap()
        .dispatch(pipeline)
        .unwrap();

    assert!((read(dir.path(), 0) - 2.0).abs() < 1e-12);
    assert!((read(dir.path(), 1) - 6.0).abs() < 1e-12);
    assert!((read(dir.path(), 2) - 8.0).abs() < 1e-12);
}

fn catalog(dir: &Path, n: usize) -> Catalog {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("init.txt"), "0").unwrap();
    Catalog {
        init_weights: TextDataset::new(dir.join("init.txt").to_str().unwrap()),
        checkpoints: (0..n)
            .map(|k| TextDataset::new(dir.join(format!("ckpt/epoch_{k}.txt")).to_str().unwrap()))
            .collect(),
        report: JsonDataset::new(dir.join("report.json").to_str().unwrap()),
    }
}

#[test]
fn cache_hook_keys_each_iteration_separately() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = dir.path().join(".pondcache");
    let params = || Params { target: Param(10.0), learning_rate: Param(0.25) };

    let ran = RanNodes::default();
    App::new(catalog(dir.path(), 3), params())
        .with_hooks((CacheHook::new(&cache_dir), ran.clone()))
        .execute::<PondError, _>(pipeline)
        .unwrap();
    assert_eq!(*ran.0.lock().unwrap(), ["train/0", "train/1", "train/2", "report"]);

    // Nothing changed: every iteration skips.
    let ran = RanNodes::default();
    App::new(catalog(dir.path(), 3), params())
        .with_hooks((CacheHook::new(&cache_dir), ran.clone()))
        .execute::<PondError, _>(pipeline)
        .unwrap();
    assert!(ran.0.lock().unwrap().is_empty(), "{:?}", ran.0.lock().unwrap());

    // A different init invalidates the whole chain, iteration by iteration.
    std::thread::sleep(std::time::Duration::from_millis(50));
    let ran = RanNodes::default();
    let cat = catalog(dir.path(), 3);
    std::fs::write(dir.path().join("init.txt"), "4").unwrap();
    App::new(cat, params())
        .with_hooks((CacheHook::new(&cache_dir), ran.clone()))
        .execute::<PondError, _>(pipeline)
        .unwrap();
    assert_eq!(*ran.0.lock().unwrap(), ["train/0", "train/1", "train/2", "report"]);
    assert!((read(dir.path(), 0) - 7.0).abs() < 1e-12);
}
