//! Example demonstrating pondrs running with only the no_std subset.
//!
//! This binary itself uses std (it's a regular executable), but it only
//! depends on pondrs features that work without std or alloc:
//! - CellDataset (core::cell::Cell based, no allocator needed)
//! - Param (read-only parameter)
//! - Node, Pipeline, Steps
//! - SequentialRunner with () hooks (no logging, no panic catching)

use pondrs::datasets::{CellDataset, Param};
use pondrs::error::PondError;
use pondrs::{Dataset, Node, Pipeline, SequentialRunner};
use pondrs::runners::Runner;
use serde::Serialize;

#[derive(Serialize)]
struct Catalog {
    a: CellDataset<i32>,
    b: CellDataset<i32>,
    c: CellDataset<i32>,
}

#[derive(Serialize)]
struct Params {
    scale: Param<i32>,
    offset: Param<i32>,
}

/// A node that returns Result with its natural error type (PondError here,
/// since CellDataset::load returns PondError on failure).
fn checked_square(b: i32) -> Result<(i32,), PondError> {
    if b == 0 {
        return Err(PondError::DatasetNotLoaded); // placeholder for a real check
    }
    Ok((b * b,))
}

fn main() -> Result<(), PondError> {
    let params = Params {
        scale: Param(3),
        offset: Param(10),
    };

    let catalog = Catalog {
        a: CellDataset::new(),
        b: CellDataset::new(),
        c: CellDataset::new(),
    };

    // Pipeline: scale_param -> a -> b -> c
    //   node1: a = scale * 2
    //   node2: b = a + offset
    //   node3: c = b * b  (returns Result — demonstrates error propagation)
    let pipe = (
        Node {
            name: "multiply",
            func: |v| (v * 2,),
            input: (&params.scale,),
            output: (&catalog.a,),
        },
        Pipeline {
            name: "transform",
            steps: (
                Node {
                    name: "add_offset",
                    func: |a, off| (a + off,),
                    input: (&catalog.a, &params.offset),
                    output: (&catalog.b,),
                },
                Node {
                    name: "square",
                    func: checked_square,
                    input: (&catalog.b,),
                    output: (&catalog.c,),
                },
            ),
            input: (&catalog.a, &params.offset),
            output: (&catalog.c,),
        },
    );

    // Run with no hooks, no panic catching
    let hooks = ();
    SequentialRunner.run::<PondError>(&pipe, &catalog, &params, &hooks)?;

    // Verify results: scale=3, so a=6, b=6+10=16, c=16*16=256
    let result = catalog.c.load()?;
    assert_eq!(result, 256);
    println!("Pipeline result: {result}");
    println!("no_std-compatible pipeline executed successfully!");
    Ok(())
}
