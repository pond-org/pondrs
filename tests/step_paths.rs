//! Hooks see each step under its full path, under both runners.

use std::sync::{Arc, Mutex};

use pondrs::datasets::{Dataset, MemoryDataset, Param};
use pondrs::error::PondError;
use pondrs::hooks::{Hook, HookAbort, HookControl};
use pondrs::pipeline::{DatasetRef, StepMeta};
use pondrs::runners::{ParallelRunner, Runner, SequentialRunner};
use pondrs::{Node, Pipeline, Steps, StepsMeta};
use serde::Serialize;

#[derive(Serialize)]
struct Catalog {
    a: MemoryDataset<i32>,
    b: MemoryDataset<i32>,
    c: MemoryDataset<i32>,
}

#[derive(Serialize)]
struct Params {
    x: Param<i32>,
}

/// Two groups nested under `outer`, each holding a node named `step`.
fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (Pipeline {
        name: "outer",
        steps: (
            Pipeline {
                name: "left",
                steps: (Node { name: "step", func: |v: i32| (v + 1,), input: (&params.x,), output: (&cat.a,) },),
                input: (&params.x,),
                output: (&cat.a,),
            },
            Pipeline {
                name: "right",
                steps: (Node { name: "step", func: |v: i32| (v * 2,), input: (&cat.a,), output: (&cat.b,) },),
                input: (&cat.a,),
                output: (&cat.b,),
            },
            Node { name: "last", func: |v: i32| (v,), input: (&cat.b,), output: (&cat.c,) },
        ),
        input: (&params.x,),
        output: (&cat.c,),
    },)
}

/// Records `(event, step name)` for every node, pipeline and save event.
#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<(&'static str, String)>>>);

impl Recorder {
    fn push(&self, event: &'static str, step: &dyn StepMeta) {
        self.0.lock().unwrap().push((event, step.name().to_string()));
    }

    fn names(&self, event: &str) -> Vec<String> {
        let mut names: Vec<String> = self.0.lock().unwrap().iter()
            .filter(|(e, _)| *e == event)
            .map(|(_, n)| n.clone())
            .collect();
        // The parallel runner's order is not definition order.
        names.sort();
        names
    }
}

impl Hook for Recorder {
    fn before_pipeline_run(&self, p: &dyn StepMeta) -> Result<HookControl, HookAbort> {
        self.push("pipeline", p);
        Ok(HookControl::Continue)
    }

    fn before_node_run(&self, n: &dyn StepMeta) -> Result<HookControl, HookAbort> {
        self.push("node", n);
        Ok(HookControl::Continue)
    }

    fn after_dataset_saved(&self, n: &dyn StepMeta, _ds: &DatasetRef) -> Result<(), HookAbort> {
        self.push("saved", n);
        Ok(())
    }
}

fn run_with(runner: &impl Runner) -> Recorder {
    let cat = Catalog { a: MemoryDataset::new(), b: MemoryDataset::new(), c: MemoryDataset::new() };
    let params = Params { x: Param(1) };
    let pipe = pipeline(&cat, &params);
    pipe.check().unwrap();

    let rec = Recorder::default();
    runner.run::<PondError>(&pipe, &cat, &params, &(rec.clone(),)).unwrap();
    assert_eq!(cat.c.load().unwrap(), 4);
    rec
}

fn assert_paths(rec: &Recorder) {
    let nodes = ["outer/last", "outer/left/step", "outer/right/step"];
    assert_eq!(rec.names("node"), nodes);
    assert_eq!(rec.names("saved"), nodes);
    assert_eq!(rec.names("pipeline"), ["outer", "outer/left", "outer/right"]);
}

#[test]
fn sequential_runner_reports_paths() {
    assert_paths(&run_with(&SequentialRunner));
}

#[test]
fn parallel_runner_reports_paths() {
    assert_paths(&run_with(&ParallelRunner::default()));
}
