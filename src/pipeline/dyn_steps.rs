//! Type-erased, heap-allocated steps via [`DynSteps`].

use std::prelude::v1::*;

use crate::error::PondError;

use super::steps::{StepsMeta, Steps};
use super::traits::{StepMeta, Step};

/// A type-erased, heap-allocated sequence of pipeline steps.
///
/// Useful when the number or types of steps are not known at compile time,
/// or when steps must be constructed dynamically at runtime (e.g. conditional
/// inclusion based on config flags).
///
/// Use [`Step::boxed()`] to box individual steps:
///
/// ```rust,ignore
/// fn pipeline<'a>(cat: &'a Catalog, flags: &Flags) -> DynSteps<'a> {
///     let mut steps: DynSteps<'a> = vec![
///         Node { name: "a", ... }.boxed(),
///         Node { name: "b", ... }.boxed(),
///     ];
///     if flags.optional {
///         steps.push(Node { name: "c", ... }.boxed());
///     }
///     steps
/// }
/// ```
pub type DynSteps<'a, E = PondError> = Vec<Box<dyn Step<E> + Send + Sync + 'a>>;

impl<E> StepsMeta for Vec<Box<dyn Step<E> + Send + Sync + '_>> {
    fn for_each_meta<'s>(&'s self, f: &mut dyn FnMut(&'s dyn StepMeta)) {
        for item in self {
            f(item.as_ref() as &dyn StepMeta);
        }
    }
}

impl<E> Steps<E> for Vec<Box<dyn Step<E> + Send + Sync + '_>> {
    fn for_each_step<'s>(&'s self, f: &mut dyn FnMut(&'s dyn Step<E>)) {
        for item in self {
            f(item.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::{CellDataset, Param};
    use crate::pipeline::Node;

    #[test]
    fn dyn_steps_iterates_steps() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        let steps: DynSteps<PondError> = vec![
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&a,) }.boxed(),
            Node { name: "n2", func: |v| (v,), input: (&a,), output: (&b,) }.boxed(),
        ];

        let mut count = 0;
        steps.for_each_step(&mut |_| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn dyn_steps_for_each_meta_yields_names() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        let steps: DynSteps<PondError> = vec![
            Node { name: "first", func: |v| (v,), input: (&p,), output: (&a,) }.boxed(),
            Node { name: "second", func: |v| (v,), input: (&a,), output: (&b,) }.boxed(),
        ];

        let mut names = Vec::new();
        steps.for_each_meta(&mut |info| names.push(info.name()));
        assert_eq!(names, ["first", "second"]);
    }

    #[test]
    fn dyn_steps_check_valid() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        let steps: DynSteps<PondError> = vec![
            Node { name: "n1", func: |v| (v,), input: (&p,), output: (&a,) }.boxed(),
            Node { name: "n2", func: |v| (v,), input: (&a,), output: (&b,) }.boxed(),
        ];

        assert!(steps.check().is_ok());
    }

    #[test]
    fn dyn_steps_check_out_of_order() {
        let p = Param(1i32);
        let a = CellDataset::<i32>::new();
        let b = CellDataset::<i32>::new();

        // n1 reads b, but b is produced by n2 which comes after
        let steps: DynSteps<PondError> = vec![
            Node { name: "n1", func: |v| (v,), input: (&b,), output: (&a,) }.boxed(),
            Node { name: "n2", func: |v| (v,), input: (&p,), output: (&b,) }.boxed(),
        ];

        let err = steps.check().unwrap_err();
        assert!(matches!(err, crate::CheckError::InputNotProduced { node_name: "n1", .. }));
    }
}
