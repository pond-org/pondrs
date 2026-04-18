//! Sequential pipeline validation (no_std compatible).

use super::id_set::IdSet;
use super::traits::{DatasetRef, StepInfo};
pub use crate::CheckError;

/// Collect all output dataset IDs from all leaf nodes (recursively).
pub(crate) fn collect_all_outputs<const N: usize>(
    item: &dyn StepInfo,
    all_produced: &mut IdSet<N>,
) {
    if item.is_leaf() {
        item.for_each_output(&mut |d: &DatasetRef| {
            all_produced.insert(d.id);
        });
    } else {
        item.for_each_child(&mut |child| {
            collect_all_outputs::<N>(child, all_produced);
        });
    }
}

/// Validate a single pipeline item recursively.
///
/// `all_produced` is the set of all datasets produced anywhere in the top-level
/// pipeline — used to distinguish external inputs (not produced by anyone, valid)
/// from misordered inputs (produced by a later node, invalid).
///
/// `produced` tracks what has been produced by earlier nodes so far.
/// `consumed` tracks what has been consumed (for pipeline contract checks).
pub(crate) fn check_item<const N: usize>(
    item: &dyn StepInfo,
    all_produced: &IdSet<N>,
    produced: &mut IdSet<N>,
    consumed: &mut IdSet<N>,
) -> Result<(), CheckError> {
    if item.is_leaf() {
        check_leaf::<N>(item, all_produced, produced, consumed)
    } else {
        check_pipeline::<N>(item, all_produced, produced, consumed)
    }
}

fn check_leaf<const N: usize>(
    item: &dyn StepInfo,
    all_produced: &IdSet<N>,
    produced: &mut IdSet<N>,
    consumed: &mut IdSet<N>,
) -> Result<(), CheckError> {
    let name = item.name();

    // Check inputs: if a dataset is produced somewhere in this pipeline
    // but not yet by an earlier node, it's an ordering error.
    // Datasets not produced by anyone are external inputs — valid.
    let mut input_err: Result<(), CheckError> = Ok(());
    item.for_each_input(&mut |d: &DatasetRef| {
        if input_err.is_err() {
            return;
        }
        if !consumed.insert(d.id) {
            input_err = Err(CheckError::CapacityExceeded);
            return;
        }
        if !d.meta.is_param() && all_produced.contains(d.id) && !produced.contains(d.id) {
            input_err = Err(CheckError::InputNotProduced {
                node_name: name,
                dataset_id: d.id,
            });
        }
    });
    input_err?;

    // Check outputs: no params, no duplicates.
    let mut output_err: Result<(), CheckError> = Ok(());
    item.for_each_output(&mut |d: &DatasetRef| {
        if output_err.is_err() {
            return;
        }
        if d.meta.is_param() {
            output_err = Err(CheckError::ParamWritten {
                node_name: name,
                dataset_id: d.id,
            });
            return;
        }
        if produced.contains(d.id) {
            output_err = Err(CheckError::DuplicateOutput {
                node_name: name,
                dataset_id: d.id,
            });
            return;
        }
        if !produced.insert(d.id) {
            output_err = Err(CheckError::CapacityExceeded);
        }
    });
    output_err
}

fn check_pipeline<const N: usize>(
    item: &dyn StepInfo,
    all_produced: &IdSet<N>,
    produced: &mut IdSet<N>,
    consumed: &mut IdSet<N>,
) -> Result<(), CheckError> {
    let name = item.name();

    // Snapshot parent produced set so children can see it.
    let mut inner_produced = IdSet::<N>::new();
    if !inner_produced.copy_from(produced) {
        return Err(CheckError::CapacityExceeded);
    }
    let mut child_consumed = IdSet::<N>::new();

    // Recurse into children in definition order.
    let mut child_err: Result<(), CheckError> = Ok(());
    item.for_each_child(&mut |child| {
        if child_err.is_ok() {
            child_err = check_item::<N>(child, all_produced, &mut inner_produced, &mut child_consumed);
        }
    });
    child_err?;

    // Merge newly produced datasets back into parent.
    if !produced.copy_from(&inner_produced) {
        return Err(CheckError::CapacityExceeded);
    }
    // Merge child consumed into parent consumed.
    if !consumed.copy_from(&child_consumed) {
        return Err(CheckError::CapacityExceeded);
    }

    // Check pipeline contract: declared outputs must be produced by children.
    let mut output_err: Result<(), CheckError> = Ok(());
    item.for_each_output(&mut |d: &DatasetRef| {
        if output_err.is_err() {
            return;
        }
        if !d.meta.is_param() && !inner_produced.contains(d.id) {
            output_err = Err(CheckError::UnproducedPipelineOutput {
                pipeline_name: name,
                dataset_id: d.id,
            });
        }
    });
    output_err?;

    // Check pipeline contract: declared inputs must be consumed by children.
    let mut input_err: Result<(), CheckError> = Ok(());
    let mut declared_inputs = IdSet::<N>::new();
    item.for_each_input(&mut |d: &DatasetRef| {
        if input_err.is_err() {
            return;
        }
        if !declared_inputs.insert(d.id) {
            input_err = Err(CheckError::CapacityExceeded);
            return;
        }
        if !child_consumed.contains(d.id) {
            input_err = Err(CheckError::UnusedPipelineInput {
                pipeline_name: name,
                dataset_id: d.id,
            });
        }
    });
    input_err?;

    // Check that children don't consume external datasets not declared in pipeline inputs.
    // External = consumed but not produced internally and not a param.
    // We need to walk child_consumed and check each against inner_produced + declared_inputs.
    // Since IdSet doesn't expose iteration, we re-walk children to find their inputs.
    let mut undeclared_err: Result<(), CheckError> = Ok(());
    item.for_each_child(&mut |child| {
        if undeclared_err.is_err() {
            return;
        }
        check_undeclared_inputs::<N>(child, &inner_produced, &declared_inputs, name, &mut undeclared_err);
    });
    undeclared_err
}

/// Recursively walk a step's inputs to find any external dataset not declared
/// in the parent pipeline's inputs.
fn check_undeclared_inputs<const N: usize>(
    item: &dyn StepInfo,
    inner_produced: &IdSet<N>,
    declared_inputs: &IdSet<N>,
    pipeline_name: &'static str,
    err: &mut Result<(), CheckError>,
) {
    if err.is_err() {
        return;
    }
    if item.is_leaf() {
        item.for_each_input(&mut |d: &DatasetRef| {
            if err.is_err() {
                return;
            }
            // Skip params, internally produced datasets, and declared inputs
            if d.meta.is_param() || inner_produced.contains(d.id) || declared_inputs.contains(d.id) {
                return;
            }
            *err = Err(CheckError::UndeclaredPipelineInput {
                pipeline_name,
                dataset_id: d.id,
            });
        });
    } else {
        // For nested pipelines, only check their declared inputs (not their internals —
        // those are validated when the nested pipeline itself is checked).
        item.for_each_input(&mut |d: &DatasetRef| {
            if err.is_err() {
                return;
            }
            if d.meta.is_param() || inner_produced.contains(d.id) || declared_inputs.contains(d.id) {
                return;
            }
            *err = Err(CheckError::UndeclaredPipelineInput {
                pipeline_name,
                dataset_id: d.id,
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{Node, Pipeline, PipelineInfo};
    use crate::datasets::{CellDataset, Param};

    #[test]
    fn valid_linear_pipeline() {
        let params = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&a,), output: (&b,) },
        );
        assert!(pipe.check().is_ok());
    }

    #[test]
    fn valid_diamond_pipeline() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let c = CellDataset::<i32>::new();

        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&p,), output: (&b,) },
            Node { name: "n3", func: |a, b| (a + b,), input: (&a, &b), output: (&c,) },
        );
        assert!(pipe.check().is_ok());
    }

    #[test]
    fn external_input_is_valid() {
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        // n1 reads a, which no node produces — it's an external input, not an error
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&a,), output: (&b,) },
        );
        assert!(pipe.check().is_ok());
    }

    #[test]
    fn out_of_order_dependency() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        // n1 reads b, but b is produced by n2 which comes after
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&b,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&p,), output: (&b,) },
        );
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::InputNotProduced { node_name: "n1", .. }));
    }

    #[test]
    fn duplicate_output() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();

        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&p,), output: (&a,) },
        );
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::DuplicateOutput { node_name: "n2", .. }));
    }

    #[test]
    fn param_written() {
        let p = Param(1i32);

        // n1 writes to param p
        let pipe = (
            Node { name: "n1", func: || ((),), input: (), output: (&p,) },
        );
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::ParamWritten { node_name: "n1", .. }));
    }

    #[test]
    fn valid_nested_pipeline() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let c = CellDataset::<i32>::new();

        let pipe = (
            Node { name: "n0", func: |v| (v,), input: (&p,), output: (&a,) },
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&a,), output: (&b,) },
                    Node { name: "n2", func: |v| (v,), input: (&b,), output: (&c,) },
                ),
                input: (&a,),
                output: (&c,),
            },
        );
        assert!(pipe.check().is_ok());
    }

    #[test]
    fn unproduced_pipeline_output() {
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let c = CellDataset::<i32>::new();

        // Pipeline declares output c, but children only produce b
        let pipe = (
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&a,), output: (&b,) },
                ),
                input: (&a,),
                output: (&c,),
            },
        );
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::UnproducedPipelineOutput { pipeline_name: "inner", .. }));
    }

    #[test]
    fn unused_pipeline_input() {
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let c = CellDataset::<i32>::new();

        // Pipeline declares input a, but children only read from b
        let pipe = (
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&b,), output: (&c,) },
                ),
                input: (&a,),
                output: (&c,),
            },
        );
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::UnusedPipelineInput { pipeline_name: "inner", .. }));
    }

    #[test]
    fn nested_pipeline_sees_outer_produced() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        // n0 produces a, inner pipeline's n1 reads a (produced outside)
        let pipe = (
            Node { name: "n0", func: |v| (v,), input: (&p,), output: (&a,) },
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&a,), output: (&b,) },
                ),
                input: (&a,),
                output: (&b,),
            },
        );
        assert!(pipe.check().is_ok());
    }

    #[test]
    fn node_after_pipeline_sees_inner_produced() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let c = CellDataset::<i32>::new();

        // inner produces b, n_after reads b
        let pipe = (
            Node { name: "n0", func: |v| (v,), input: (&p,), output: (&a,) },
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&a,), output: (&b,) },
                ),
                input: (&a,),
                output: (&b,),
            },
            Node { name: "n_after", func: |v| (v,), input: (&b,), output: (&c,) },
        );
        assert!(pipe.check().is_ok());
    }

    #[test]
    fn undeclared_pipeline_input() {
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        // Pipeline child reads `a` from outside, but pipeline doesn't declare it as input
        let pipe = (
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&a,), output: (&b,) },
                ),
                input: (),
                output: (&b,),
            },
        );
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::UndeclaredPipelineInput { pipeline_name: "inner", .. }));
    }

    #[test]
    fn check_with_capacity_works() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();

        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&a,) },
        );
        assert!(pipe.check_with_capacity::<4>().is_ok());
    }

    #[test]
    fn capacity_exceeded() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        // N=1 can only hold 1 dataset, but we produce 2
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&p,), output: (&b,) },
        );
        let err = pipe.check_with_capacity::<1>().unwrap_err();
        assert!(matches!(err, CheckError::CapacityExceeded));
    }
}
