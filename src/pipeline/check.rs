//! Sequential pipeline validation (`no_std` compatible).

use super::id_set::{IdCollector, TypeCollector, TypeInsert};
use super::traits::{DatasetRef, StepMeta};
use crate::error::CheckWarning;
use crate::naming::{is_leaf_type, type_ident};
pub use crate::CheckError;

/// Visits a set of sibling steps: a group's children, or the top-level steps.
type ForEachSibling<'s, 'a> = &'s dyn Fn(&mut dyn FnMut(&'a dyn StepMeta));

/// Reject step names that would make two step paths coincide: a name
/// containing `/`, or two siblings sharing a name. Recurses into groups.
///
/// `for_each` iterates the siblings and `group` names their parent (`None` at the top level). Siblings
/// are compared pairwise rather than collected, so this needs no allocator.
pub(crate) fn check_names<'a>(
    group: Option<&'a str>,
    for_each: ForEachSibling<'_, 'a>,
) -> Result<(), CheckError<'a>> {
    let mut err: Result<(), CheckError<'a>> = Ok(());
    let mut index = 0;
    for_each(&mut |step| {
        if err.is_err() {
            return;
        }
        let name = step.name();
        if name.contains('/') {
            err = Err(CheckError::InvalidStepName { name });
            return;
        }
        let mut earlier = 0;
        let mut duplicate = false;
        for_each(&mut |other| {
            duplicate |= earlier < index && other.name() == name;
            earlier += 1;
        });
        if duplicate {
            err = Err(CheckError::DuplicateStepName { group, name });
            return;
        }
        index += 1;
        if !step.is_leaf() {
            err = check_names(Some(name), &|f| step.for_each_child(f));
        }
    });
    err
}

/// Detect two datasets of different types sharing one pointer id.
///
/// Runs before the ordering checks: an alias makes two distinct datasets look
/// like one, which produces spurious `DuplicateOutput` / `InputNotProduced`
/// errors downstream. Reporting it first keeps the user off a phantom trail.
///
/// Groups contribute nothing of their own — their declared input/output refs
/// duplicate their children's and would attribute the conflict to the wrong
/// step.
pub(crate) fn check_dataset_identity<'a, T: TypeCollector>(
    item: &'a dyn StepMeta,
    seen: &mut T,
) -> Result<(), CheckError<'a>> {
    if !item.is_leaf() {
        let mut child_err: Result<(), CheckError<'a>> = Ok(());
        item.for_each_child(&mut |child| {
            if child_err.is_ok() {
                child_err = check_dataset_identity(child, seen);
            }
        });
        return child_err;
    }

    let name = item.name();
    let mut err: Result<(), CheckError<'a>> = Ok(());
    let mut record = |d: &DatasetRef| {
        if err.is_err() {
            return;
        }
        let ty = d.meta.type_string();
        match seen.insert(d.id, ty) {
            TypeInsert::Inserted | TypeInsert::Match => {}
            TypeInsert::Conflict(other) => {
                err = Err(CheckError::AliasedDatasets {
                    node_name: name,
                    dataset_id: d.id,
                    type_name: ty,
                    conflicting_type_name: other,
                });
            }
            TypeInsert::Full => err = Err(CheckError::CapacityExceeded),
        }
    };
    item.for_each_input(&mut record);
    item.for_each_output(&mut record);
    err
}

/// Report non-fatal naming/identity diagnostics for one step, recursively.
///
/// `seen` dedupes by dataset id so a dataset consumed by several nodes warns
/// once. If it fills up, deduping stops but reporting continues — dropping a
/// warning is worse than repeating one, and there is no error path here.
pub(crate) fn collect_warnings<C: IdCollector>(
    item: &dyn StepMeta,
    seen: &mut C,
    report: &mut dyn FnMut(&CheckWarning),
) {
    if !item.is_leaf() {
        item.for_each_child(&mut |child| collect_warnings(child, seen, report));
        return;
    }

    let name = item.name();
    let mut check_ref = |d: &DatasetRef| {
        if seen.contains(d.id) {
            return;
        }
        seen.insert(d.id);

        let ty = d.meta.type_string();
        if d.meta.is_zero_sized() {
            report(&CheckWarning::ZeroSizedDataset {
                node_name: name,
                dataset_id: d.id,
                type_name: ty,
            });
        }
        // Params are recognised by the indexer under their own name, so the
        // `*Dataset` suffix does not apply to them.
        if !d.meta.is_param() && !is_leaf_type(type_ident(ty)) {
            report(&CheckWarning::UnconventionalDatasetType {
                node_name: name,
                dataset_id: d.id,
                type_name: ty,
            });
        }
    };
    item.for_each_input(&mut check_ref);
    item.for_each_output(&mut check_ref);
}

/// Collect all output dataset IDs from all leaf nodes (recursively).
pub(crate) fn collect_all_outputs<C: IdCollector>(
    item: &dyn StepMeta,
    all_produced: &mut C,
) {
    if item.is_leaf() {
        item.for_each_output(&mut |d: &DatasetRef| {
            all_produced.insert(d.id);
        });
    } else {
        item.for_each_child(&mut |child| {
            collect_all_outputs(child, all_produced);
        });
    }
}

/// Validate a single step recursively.
///
/// `all_produced` is the set of all datasets produced anywhere in the top-level
/// pipeline — used to distinguish external inputs (not produced by anyone, valid)
/// from misordered inputs (produced by a later node, invalid).
///
/// `produced` tracks what has been produced by earlier nodes so far.
/// `consumed` tracks what has been consumed (for pipeline contract checks).
pub(crate) fn check_item<'a, C: IdCollector>(
    item: &'a dyn StepMeta,
    all_produced: &C,
    produced: &mut C,
    consumed: &mut C,
) -> Result<(), CheckError<'a>> {
    if item.is_leaf() {
        check_leaf(item, all_produced, produced, consumed)
    } else {
        check_pipeline(item, all_produced, produced, consumed)
    }
}

fn check_leaf<'a, C: IdCollector>(
    item: &'a dyn StepMeta,
    all_produced: &C,
    produced: &mut C,
    consumed: &mut C,
) -> Result<(), CheckError<'a>> {
    let name = item.name();

    // Check inputs: if a dataset is produced somewhere in this pipeline
    // but not yet by an earlier node, it's an ordering error.
    // Datasets not produced by anyone are external inputs — valid.
    let mut input_err: Result<(), CheckError<'a>> = Ok(());
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
    let mut output_err: Result<(), CheckError<'a>> = Ok(());
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

fn check_pipeline<'a, C: IdCollector>(
    item: &'a dyn StepMeta,
    all_produced: &C,
    produced: &mut C,
    consumed: &mut C,
) -> Result<(), CheckError<'a>> {
    let name = item.name();

    // Snapshot parent produced set so children can see it.
    let mut inner_produced = C::empty();
    if !inner_produced.copy_from(produced) {
        return Err(CheckError::CapacityExceeded);
    }
    let mut child_consumed = C::empty();

    // Recurse into children in definition order.
    let mut child_err: Result<(), CheckError<'a>> = Ok(());
    item.for_each_child(&mut |child| {
        if child_err.is_ok() {
            child_err = check_item(child, all_produced, &mut inner_produced, &mut child_consumed);
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
    let mut output_err: Result<(), CheckError<'a>> = Ok(());
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
    let mut input_err: Result<(), CheckError<'a>> = Ok(());
    let mut declared_inputs = C::empty();
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
    // Since IdCollector doesn't expose iteration, we re-walk children to find their inputs.
    let mut undeclared_err: Result<(), CheckError<'a>> = Ok(());
    item.for_each_child(&mut |child| {
        if undeclared_err.is_err() {
            return;
        }
        check_undeclared_inputs(child, &inner_produced, &declared_inputs, name, &mut undeclared_err);
    });
    undeclared_err
}

/// Recursively walk a step's inputs to find any external dataset not declared
/// in the parent pipeline's inputs.
fn check_undeclared_inputs<'a, C: IdCollector>(
    item: &dyn StepMeta,
    inner_produced: &C,
    declared_inputs: &C,
    pipeline_name: &'a str,
    err: &mut Result<(), CheckError<'a>>,
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
    use crate::pipeline::{Node, Pipeline, StepsMeta};
    use crate::datasets::{CellDataset, Dataset, Param};
    use serde::ser::{Serialize, Serializer};

    /// A zero-sized dataset type. Two of these as adjacent catalog fields land
    /// at one address, which is the whole point of the fixture.
    struct ZstOneDataset;
    /// A second, distinct zero-sized dataset type.
    struct ZstTwoDataset;
    /// A non-zero-sized dataset whose ident breaks the `*Dataset` convention.
    struct MyStore {
        tag: u32,
    }

    macro_rules! trivial_dataset {
        ($t:ty, $val:expr) => {
            impl Serialize for $t {
                fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                    s.serialize_unit()
                }
            }
            impl Dataset for $t {
                type LoadItem = i32;
                type SaveItem = i32;
                type Error = crate::error::PondError;
                fn load(&self) -> Result<i32, Self::Error> {
                    Ok($val)
                }
                fn save(&self, _output: i32) -> Result<(), Self::Error> {
                    Ok(())
                }
            }
        };
    }

    trivial_dataset!(ZstOneDataset, 1);
    trivial_dataset!(ZstTwoDataset, 2);

    impl Serialize for MyStore {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_u32(self.tag)
        }
    }
    impl Dataset for MyStore {
        type LoadItem = i32;
        type SaveItem = i32;
        type Error = crate::error::PondError;
        fn load(&self) -> Result<i32, Self::Error> {
            Ok(i32::try_from(self.tag).unwrap_or(i32::MAX))
        }
        fn save(&self, _output: i32) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct ZstCatalog {
        one: ZstOneDataset,
        two: ZstTwoDataset,
    }

    /// Count warnings by variant: (zero-sized, unconventional).
    fn warning_counts(pipe: &impl StepsMeta) -> (usize, usize) {
        let mut zero = 0;
        let mut unconventional = 0;
        pipe.for_each_warning(&mut |w| match w {
            CheckWarning::ZeroSizedDataset { .. } => zero += 1,
            CheckWarning::UnconventionalDatasetType { .. } => unconventional += 1,
        });
        (zero, unconventional)
    }

    #[test]
    fn aliased_zero_sized_datasets_are_rejected() {
        let p = Param(1i32);
        let cat = ZstCatalog { one: ZstOneDataset, two: ZstTwoDataset };

        // The premise: two zero-sized fields share one address.
        assert_eq!(super::super::ptr_to_id(&cat.one), super::super::ptr_to_id(&cat.two));

        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&cat.one,) },
            Node { name: "n2", func: |v| (v,), input: (&p,), output: (&cat.two,) },
        );
        let err = pipe.check().unwrap_err();
        // Reported as aliasing, not as the `DuplicateOutput` it would otherwise
        // masquerade as.
        assert!(matches!(err, CheckError::AliasedDatasets { node_name: "n2", .. }), "got {err:?}");
    }

    #[test]
    fn zero_sized_dataset_warns() {
        let p = Param(1i32);
        let ds = ZstOneDataset;

        let pipe = (Node { name: "n1", func: |v| (v,), input: (&p,), output: (&ds,) },);
        let (zero, _) = warning_counts(&pipe);
        assert_eq!(zero, 1);
    }

    #[test]
    fn unconventional_dataset_type_warns() {
        let p = Param(1i32);
        let store = MyStore { tag: 7 };

        let pipe = (Node { name: "n1", func: |v| (v,), input: (&p,), output: (&store,) },);
        let mut seen: Option<&'static str> = None;
        pipe.for_each_warning(&mut |w| {
            if let CheckWarning::UnconventionalDatasetType { type_name, .. } = w {
                seen = Some(type_name);
            }
        });
        assert!(seen.is_some_and(|t| t.ends_with("MyStore")), "got {seen:?}");
    }

    #[test]
    fn conventional_pipeline_warns_about_nothing() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&a,), output: (&b,) },
        );
        let mut count = 0;
        pipe.for_each_warning(&mut |_| count += 1);
        assert_eq!(count, 0);
    }

    #[test]
    fn warnings_are_deduped_per_dataset() {
        let p = Param(1i32);
        let store = MyStore { tag: 1 };
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let c = CellDataset::<i32>::new();

        // `store` is read by three nodes but must warn only once.
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&store,), output: (&a,) },
            Node { name: "n2", func: |v| (v,), input: (&store,), output: (&b,) },
            Node { name: "n3", func: |v| (v,), input: (&store,), output: (&c,) },
        );
        let _ = &p;
        let (_, unconventional) = warning_counts(&pipe);
        assert_eq!(unconventional, 1);
    }

    #[test]
    fn warnings_recurse_into_nested_pipelines() {
        let p = Param(1i32);
        let store = MyStore { tag: 1 };
        let a = CellDataset::<i32>::new();

        let pipe = (Pipeline {
            name: "inner",
            steps: (Node { name: "n1", func: |v| (v,), input: (&store,), output: (&a,) },),
            input: (&store,),
            output: (&a,),
        },);
        let _ = &p;
        let (_, unconventional) = warning_counts(&pipe);
        assert_eq!(unconventional, 1);
    }

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

    #[test]
    fn duplicate_top_level_names_are_rejected() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let pipe = (
            Node { name: "n", func: |v| (v,), input: (&p,), output: (&a,) },
            Node { name: "n", func: |v| (v,), input: (&p,), output: (&b,) },
        );
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::DuplicateStepName { group: None, name: "n" }), "got {err:?}");
    }

    #[test]
    fn duplicate_names_within_a_group_are_rejected() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let pipe = (Pipeline {
            name: "g",
            steps: (
                Node { name: "n", func: |v| (v,), input: (&p,), output: (&a,) },
                Node { name: "n", func: |v| (v,), input: (&p,), output: (&b,) },
            ),
            input: (&p,),
            output: (&a, &b),
        },);
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::DuplicateStepName { group: Some("g"), name: "n" }), "got {err:?}");
    }

    #[test]
    fn same_name_in_different_groups_is_allowed() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();
        let pipe = (
            Pipeline {
                name: "g1",
                steps: (Node { name: "n", func: |v| (v,), input: (&p,), output: (&a,) },),
                input: (&p,),
                output: (&a,),
            },
            Pipeline {
                name: "g2",
                steps: (Node { name: "n", func: |v| (v,), input: (&p,), output: (&b,) },),
                input: (&p,),
                output: (&b,),
            },
        );
        pipe.check().unwrap();
    }

    #[test]
    fn slash_in_a_step_name_is_rejected() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let pipe = (Node { name: "a/b", func: |v| (v,), input: (&p,), output: (&a,) },);
        let err = pipe.check().unwrap_err();
        assert!(matches!(err, CheckError::InvalidStepName { name: "a/b" }), "got {err:?}");
    }
}
