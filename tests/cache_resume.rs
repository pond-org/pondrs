//! `CacheHook` resuming a multi-stage pipeline: preprocessing, a recurrent
//! training loop with per-epoch validation and pruned checkpoints, and a
//! report. A run is interrupted at a chosen node, and the resumed run must
//! skip exactly what finished and produce what an uninterrupted run would.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use pondrs::app::App;
use pondrs::datasets::{Param, TextDataset};
use pondrs::error::PondError;
use pondrs::hooks::RetentionHook;
use pondrs::{CacheHook, Hook, HookAbort, HookControl, Node, ParallelRunner, RecurrentPipeline, StepMeta, Steps};

// ---------------------------------------------------------------------------
// The pipeline
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Epoch {
    weights: TextDataset,
    metrics: TextDataset,
}

#[derive(Serialize)]
struct Catalog {
    raw: TextDataset,
    prepared: TextDataset,
    init: Epoch,
    epochs: Vec<Epoch>,
    report: TextDataset,
}

#[derive(Serialize)]
struct Params {
    scale: Param<f64>,
    lr: Param<f64>,
    /// Read by validation only.
    tolerance: Param<f64>,
}

const EPOCHS: usize = 4;

fn params(tolerance: f64) -> Params {
    Params { scale: Param(2.0), lr: Param(0.5), tolerance: Param(tolerance) }
}

fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    let last = cat.epochs.last().expect("at least one epoch");
    (
        Node {
            name: "prep",
            input: (&cat.raw, &params.scale),
            output: (&cat.prepared,),
            func: |raw: String, scale: f64| ((num(&raw) * scale).to_string(),),
        },
        RecurrentPipeline {
            name: "epochs",
            state: &cat.epochs,
            init: &cat.init,
            input: |prev, _cur| (&prev.weights, &cat.prepared, &params.lr, &params.tolerance),
            output: |cur| (&cur.weights, &cur.metrics),
            pipe: |prev, cur| (
                Node {
                    name: "train",
                    input: (&prev.weights, &cat.prepared, &params.lr),
                    output: (&cur.weights,),
                    func: |w: String, t: String, lr: f64| {
                        let w = num(&w);
                        ((w + lr * (num(&t) - w)).to_string(),)
                    },
                },
                Node {
                    name: "val",
                    input: (&cur.weights, &cat.prepared, &params.tolerance),
                    output: (&cur.metrics,),
                    func: |w: String, t: String, tol: f64| {
                        let err = (num(&t) - num(&w)).abs();
                        (format!("{err} {}", err < tol),)
                    },
                },
            ),
        }
        .build(),
        Node {
            name: "report",
            input: (&last.weights, &last.metrics),
            output: (&cat.report,),
            func: |w: String, m: String| (format!("weights={w} metrics={m}"),),
        },
    )
}

fn num(s: &str) -> f64 {
    s.trim().parse().unwrap()
}

/// A catalog writing its outputs under `dir/out`. Inputs are seeded once:
/// `CacheHook` hashes files by mtime, so rewriting them would be a change.
fn catalog(dir: &Path, out: &str) -> Catalog {
    let file = |p: PathBuf| TextDataset::new(p.to_str().unwrap());
    let o = dir.join(out);
    std::fs::create_dir_all(&o).unwrap();
    for (name, value) in [("raw.txt", "5"), ("init.txt", "0")] {
        if !dir.join(name).exists() {
            std::fs::write(dir.join(name), value).unwrap();
        }
    }
    Catalog {
        raw: file(dir.join("raw.txt")),
        prepared: file(o.join("prepared.txt")),
        init: Epoch { weights: file(dir.join("init.txt")), metrics: file(o.join("init_metrics.txt")) },
        epochs: (0..EPOCHS)
            .map(|k| Epoch {
                weights: file(o.join(format!("weights_{k}.txt"))),
                metrics: file(o.join(format!("metrics_{k}.txt"))),
            })
            .collect(),
        report: file(o.join("report.txt")),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Records the path of every node that ran (not skipped), sorted.
#[derive(Clone, Default)]
struct Ran(Arc<Mutex<Vec<String>>>);

impl Ran {
    fn sorted(&self) -> Vec<String> {
        let mut v = self.0.lock().unwrap().clone();
        v.sort();
        v
    }
}

impl Hook for Ran {
    fn after_node_run(&self, n: &dyn StepMeta, skipped: bool) -> Result<(), HookAbort> {
        if !skipped {
            self.0.lock().unwrap().push(n.name().to_string());
        }
        Ok(())
    }
}

/// Stops the run before the node at `path`, like a crash there.
struct CrashAt(&'static str);

impl Hook for CrashAt {
    fn before_node_run(&self, n: &dyn StepMeta) -> Result<HookControl, HookAbort> {
        if n.name() == self.0 { Err(HookAbort("simulated crash")) } else { Ok(HookControl::Continue) }
    }
}

fn retention() -> RetentionHook {
    RetentionHook::new("catalog.epochs.*.weights", 1)
}

fn run_seq(dir: &Path, out: &str, tolerance: f64, hooks: impl pondrs::Hooks) -> Result<(), PondError> {
    App::new(catalog(dir, out), params(tolerance)).with_hooks(hooks).execute::<PondError, _>(pipeline)
}

fn run_par(dir: &Path, out: &str, tolerance: f64, hooks: impl pondrs::Hooks) -> Result<(), PondError> {
    App::new(catalog(dir, out), params(tolerance))
        .with_hooks(hooks)
        .with_runners((ParallelRunner::new(2),))
        .execute::<PondError, _>(pipeline)
}

/// The report of an uninterrupted, uncached run.
fn reference_report() -> String {
    let dir = tempfile::tempdir().unwrap();
    run_seq(dir.path(), "out", 1.0, ()).unwrap();
    report(dir.path(), "out")
}

fn report(dir: &Path, out: &str) -> String {
    std::fs::read_to_string(dir.join(out).join("report.txt")).unwrap()
}

fn weights_kept(dir: &Path) -> Vec<usize> {
    (0..EPOCHS).filter(|k| dir.join(format!("out/weights_{k}.txt")).exists()).collect()
}

fn names(v: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = v.iter().map(|s| (*s).to_string()).collect();
    v.sort();
    v
}

/// Moves later mtime ticks past anything the previous run wrote.
fn tick() {
    std::thread::sleep(std::time::Duration::from_millis(50));
}

// ---------------------------------------------------------------------------
// Resuming at each stage
// ---------------------------------------------------------------------------

/// Interrupts a run before `crash_at`, resumes it, and checks that the resumed
/// run ran exactly `expect_ran` and reproduced the uninterrupted result.
fn crash_and_resume(crash_at: &'static str, expect_ran: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");

    let err = run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache), retention(), CrashAt(crash_at)));
    assert!(matches!(err, Err(PondError::HookAbort("simulated crash"))), "{err:?}");

    tick();
    let ran = Ran::default();
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache), retention(), ran.clone())).unwrap();
    assert_eq!(ran.sorted(), names(expect_ran));
    assert_eq!(report(dir.path(), "out"), reference_report());
}

#[test]
fn resume_after_crash_in_preprocessing() {
    crash_and_resume(
        "prep",
        &[
            "prep",
            "epochs/0/train", "epochs/0/val", "epochs/1/train", "epochs/1/val",
            "epochs/2/train", "epochs/2/val", "epochs/3/train", "epochs/3/val",
            "report",
        ],
    );
}

#[test]
fn resume_after_crash_mid_training() {
    // Epochs 0 and 1 finished, and their checkpoints but the newest were
    // pruned: they skip, and epoch 2 trains from the kept checkpoint.
    crash_and_resume(
        "epochs/2/train",
        &["epochs/2/train", "epochs/2/val", "epochs/3/train", "epochs/3/val", "report"],
    );
}

#[test]
fn resume_after_crash_between_train_and_val() {
    // Epoch 1 trained but did not validate: validation reads the checkpoint
    // its skipped producer left, which is the newest one and so was kept.
    crash_and_resume(
        "epochs/1/val",
        &["epochs/1/val", "epochs/2/train", "epochs/2/val", "epochs/3/train", "epochs/3/val", "report"],
    );
}

#[test]
fn resume_after_crash_in_report() {
    crash_and_resume("report", &["report"]);
}

#[test]
fn resume_with_parallel_runner() {
    // No retention: under `ParallelRunner` a validation node can lag behind
    // the next epoch's training, so `keep_last: 1` would prune its input.
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");

    let err = run_par(dir.path(), "out", 1.0, (CacheHook::new(&cache), CrashAt("epochs/2/train")));
    assert!(err.is_err());

    tick();
    let ran = Ran::default();
    run_par(dir.path(), "out", 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
    let ran = ran.sorted();
    // Whatever finished before the crash is skipped; epoch 2 onwards must run.
    for node in ["epochs/2/train", "epochs/2/val", "epochs/3/train", "epochs/3/val", "report"] {
        assert!(ran.iter().any(|r| r == node), "{node} did not run: {ran:?}");
    }
    assert!(!ran.iter().any(|r| r == "prep" || r == "epochs/0/train"), "{ran:?}");
    assert_eq!(report(dir.path(), "out"), reference_report());

    // And a third run skips everything.
    tick();
    let ran = Ran::default();
    run_par(dir.path(), "out", 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
    assert!(ran.sorted().is_empty(), "{:?}", ran.sorted());
}

// ---------------------------------------------------------------------------
// Pruned inputs, catalogs, invalidation
// ---------------------------------------------------------------------------

#[test]
fn rerun_needing_a_pruned_checkpoint_fails_fast() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache), retention())).unwrap();
    assert_eq!(weights_kept(dir.path()), [EPOCHS - 1]);

    // A validation-only change re-runs `epochs/0/val`, whose input was pruned
    // after `epochs/0/train` (now skipped) wrote it.
    tick();
    let ran = Ran::default();
    let err = run_seq(dir.path(), "out", 0.5, (CacheHook::new(&cache), retention(), ran.clone())).unwrap_err();
    let PondError::HookAbort(msg) = err else { panic!("{err:?}") };
    assert!(msg.contains("`epochs/0/val`"), "{msg}");
    assert!(msg.contains("skipped node `epochs/0/train`"), "{msg}");
    assert!(msg.contains("weights_0.txt"), "{msg}");
    assert!(ran.sorted().is_empty(), "nothing should run before the abort: {:?}", ran.sorted());
}

#[test]
fn rerun_needing_a_pruned_checkpoint_fails_fast_in_parallel() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");
    run_par(dir.path(), "out", 1.0, (CacheHook::new(&cache),)).unwrap();
    for k in 0..EPOCHS - 1 {
        std::fs::remove_file(dir.path().join(format!("out/weights_{k}.txt"))).unwrap();
    }

    tick();
    let err = run_par(dir.path(), "out", 0.5, (CacheHook::new(&cache),)).unwrap_err();
    let PondError::HookAbort(msg) = err else { panic!("{err:?}") };
    assert!(msg.contains("needs an input produced by skipped node `epochs/"), "{msg}");
}

#[test]
fn catalogs_writing_elsewhere_keep_separate_keys() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");
    let all = 2 + 2 * EPOCHS;

    // Same params, same node names, outputs in different places: the second
    // catalog must not skip on the first one's keys.
    for out in ["out", "scratch"] {
        let ran = Ran::default();
        run_seq(dir.path(), out, 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
        assert_eq!(ran.sorted().len(), all, "{out}");
    }
    assert_eq!(report(dir.path(), "scratch"), report(dir.path(), "out"));

    // Alternating between them does not invalidate either.
    tick();
    for out in ["out", "scratch", "out"] {
        let ran = Ran::default();
        run_seq(dir.path(), out, 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
        assert!(ran.sorted().is_empty(), "{out}: {:?}", ran.sorted());
    }
}

#[test]
fn deleting_a_node_directory_invalidates_it() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache),)).unwrap();

    tick();
    std::fs::remove_dir_all(cache.join("epochs/3")).unwrap();
    let ran = Ran::default();
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
    // `report`'s key depends on its producers' keys, not on their output
    // bytes, so it still skips.
    assert_eq!(ran.sorted(), names(&["epochs/3/train", "epochs/3/val"]));
}

#[test]
fn force_runs_everything_and_keeps_keys() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache),)).unwrap();

    tick();
    let ran = Ran::default();
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache).force(true), ran.clone())).unwrap();
    assert_eq!(ran.sorted().len(), 2 + 2 * EPOCHS);

    tick();
    let ran = Ran::default();
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
    assert!(ran.sorted().is_empty(), "{:?}", ran.sorted());
}

#[test]
fn old_flat_key_files_are_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join(".pondcache");
    std::fs::create_dir_all(&cache).unwrap();
    // The earlier layout: one file per node, named after the sanitized path.
    std::fs::write(cache.join("prep"), "12345").unwrap();
    std::fs::write(cache.join("epochs"), "12345").unwrap();

    let ran = Ran::default();
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
    assert_eq!(ran.sorted().len(), 2 + 2 * EPOCHS);

    tick();
    let ran = Ran::default();
    run_seq(dir.path(), "out", 1.0, (CacheHook::new(&cache), ran.clone())).unwrap();
    assert!(ran.sorted().is_empty(), "{:?}", ran.sorted());
}
