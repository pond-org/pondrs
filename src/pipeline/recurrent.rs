//! `RecurrentNode` / `RecurrentPipeline` — a loop unrolled into one real
//! [`Node`] or [`Pipeline`] per iteration.

use std::collections::HashSet;
use std::prelude::v1::*;

use super::node::{CompatibleOutput, Node};
use super::pipeline::Pipeline;
use super::stable::StableFn;
use super::steps::StepsMeta;
use super::traits::{DatasetRef, Group, NodeInputMeta, NodeOutputMeta, Step, StepKind, StepMeta};

/// A computation repeated once per element of a state vector, each iteration
/// reading the previous iteration's state and writing its own.
///
/// [`build`](Self::build) unrolls it into an [`Unrolled`] group named `name`,
/// holding one plain [`Node`] per iteration named `k` — so iteration `k`'s path
/// is `"{name}/{k}"`. Because the children are real nodes, everything that
/// works on nodes works on each iteration individually: `--from-nodes train/47` resumes mid-chain, `CacheHook` keys
/// each iteration separately, and hooks report per-iteration progress.
///
/// # The state vector is the chain
///
/// `state` holds one element per iteration, so `state.len()` *is* the
/// iteration count. Iteration `k` is handed `state[k - 1]` and `state[k]`
/// (iteration 0 gets `init` in place of `state[-1]`), and the `input` / `output`
/// projections build the node's tuples from them:
///
/// ```rust,ignore
/// // Each element is a dataset: the projection is close to the identity.
/// RecurrentNode {
///     name: "train",
///     state: &cat.weights,                   // Vec<JsonDataset>
///     init: &cat.init_weights,
///     input: |prev, _cur| (prev, &cat.features),
///     output: |cur| (cur,),
///     func: train_epoch,
/// }
/// .build()
///
/// // Each element is a sub-catalog: per-iteration extras come off `cur`.
/// RecurrentNode {
///     name: "train",
///     state: &cat.epochs,                    // Vec<EpochSlot>
///     init: &cat.init_epoch,
///     input: |prev, cur| (&prev.weights, &cat.features, &cur.lr),
///     output: |cur| (&cur.weights, &cur.metrics),
///     func: train_epoch,
/// }
/// .build()
/// ```
///
/// Each element must be a distinct dataset (or hold distinct datasets): two
/// nodes may not write one dataset. A `Vec<S>` catalog field gives exactly that,
/// and the catalog indexer names its elements `epochs.0.weights`, … — see
/// [`expand_indexed`](crate::datasets::expand_indexed) for writing one in YAML
/// without listing every element.
///
/// # Field order
///
/// As with [`Node`], write `state` and `init` before the projections and
/// `func` last: fields are type-checked in written order, and the projections'
/// closure parameters are inferred from `state`, while `func`'s signature is
/// checked against what the projections return.
///
/// `func` is cloned once per iteration, so it must be `Clone` — a plain `fn`
/// item or a non-capturing closure is.
pub struct RecurrentNode<'a, F, S, In, Out, FI, FO, N = &'static str>
where
    In: NodeInputMeta,
    Out: NodeOutputMeta,
    F: StableFn<In::Args> + Clone,
    F::Output: CompatibleOutput<Out::Output>,
    FI: Fn(&'a S, &'a S) -> In,
    FO: Fn(&'a S) -> Out,
    N: AsRef<str> + Send + Sync,
{
    /// The group's name; iteration `k` is named `k`, so its path is `"{name}/{k}"`.
    pub name: N,
    /// One element per iteration.
    pub state: &'a [S],
    /// The state iteration 0 reads as its "previous".
    pub init: &'a S,
    /// Builds an iteration's input tuple from `(prev, cur)`.
    pub input: FI,
    /// Builds an iteration's output tuple from `cur`.
    pub output: FO,
    pub func: F,
}

impl<'a, F, S, In, Out, FI, FO, N> RecurrentNode<'a, F, S, In, Out, FI, FO, N>
where
    In: NodeInputMeta,
    Out: NodeOutputMeta,
    F: StableFn<In::Args> + Clone,
    F::Output: CompatibleOutput<Out::Output>,
    FI: Fn(&'a S, &'a S) -> In,
    FO: Fn(&'a S) -> Out,
    N: AsRef<str> + Send + Sync,
{
    /// Unroll into one [`Node`] per element of `state`.
    pub fn build(self) -> Unrolled<Node<F, In, Out, String>, N> {
        let steps = unroll(self.state, self.init, |k, prev, cur| Node {
            name: k.to_string(),
            input: (self.input)(prev, cur),
            output: (self.output)(cur),
            func: self.func.clone(),
        });
        Unrolled { name: self.name, steps }
    }
}

/// A sub-pipeline repeated once per element of a state vector, each iteration
/// reading the previous iteration's state and writing its own — the
/// [`Pipeline`] counterpart of [`RecurrentNode`].
///
/// [`build`](Self::build) unrolls it into an [`Unrolled`] group named `name`,
/// holding one [`Pipeline`] per iteration named `k`. A node named `fwd` inside
/// `pipe` therefore has the path `"{name}/{k}/fwd"`: give the nodes plain
/// names, and the iteration's pipeline keeps them apart.
///
/// ```rust,ignore
/// RecurrentPipeline {
///     name: "train",
///     state: &cat.epochs,                    // Vec<EpochSlot>
///     init: &cat.init_epoch,
///     input: |prev, _cur| (&prev.weights, &cat.features),
///     output: |cur| (&cur.weights,),
///     pipe: |prev, cur| (
///         Node { name: "fwd", input: (&prev.weights, &cat.features), output: (&cur.acts,), func: forward },
///         Node { name: "bwd", input: (&prev.weights, &cur.acts), output: (&cur.weights,), func: backward },
///     ),
/// }
/// .build()
/// ```
///
/// `input` and `output` are each iteration's *declared* contract, exactly as
/// on a hand-written [`Pipeline`], and `check` holds every iteration to it:
/// a declared output its nodes do not produce, a declared input they do not
/// read, or an external dataset they read without declaring it is an error.
/// `pipe` builds the iteration's steps from `(prev, cur)` rather than from the
/// contract, because a pipeline also has intermediates (`cur.acts` above) —
/// per-iteration datasets that live in the state element but are not part of
/// the contract.
///
/// Iteration `k` is handed `state[k - 1]` and `state[k]` (iteration 0 gets
/// `init` in place of `state[-1]`), as in [`RecurrentNode`], whose docs cover
/// the state vector and the catalog side in more detail. Write `state` and
/// `init` before the closures: fields are type-checked in written order, and
/// the closures' parameters are inferred from `state`.
pub struct RecurrentPipeline<'a, S, In, Out, P, FI, FO, FP, N = &'static str>
where
    In: NodeInputMeta,
    Out: NodeOutputMeta,
    P: StepsMeta,
    FI: Fn(&'a S, &'a S) -> In,
    FO: Fn(&'a S) -> Out,
    FP: Fn(&'a S, &'a S) -> P,
    N: AsRef<str> + Send + Sync,
{
    /// The group's name; iteration `k` is named `k`, so its path is `"{name}/{k}"`.
    pub name: N,
    /// One element per iteration.
    pub state: &'a [S],
    /// The state iteration 0 reads as its "previous".
    pub init: &'a S,
    /// Builds an iteration's declared inputs from `(prev, cur)`.
    pub input: FI,
    /// Builds an iteration's declared outputs from `cur`.
    pub output: FO,
    /// Builds an iteration's steps from `(prev, cur)`.
    pub pipe: FP,
}

impl<'a, S, In, Out, P, FI, FO, FP, N> RecurrentPipeline<'a, S, In, Out, P, FI, FO, FP, N>
where
    In: NodeInputMeta,
    Out: NodeOutputMeta,
    P: StepsMeta,
    FI: Fn(&'a S, &'a S) -> In,
    FO: Fn(&'a S) -> Out,
    FP: Fn(&'a S, &'a S) -> P,
    N: AsRef<str> + Send + Sync,
{
    /// Unroll into one [`Pipeline`] per element of `state`.
    pub fn build(self) -> Unrolled<Pipeline<P, In, Out, String>, N> {
        let steps = unroll(self.state, self.init, |k, prev, cur| Pipeline {
            name: k.to_string(),
            steps: (self.pipe)(prev, cur),
            input: (self.input)(prev, cur),
            output: (self.output)(cur),
        });
        Unrolled { name: self.name, steps }
    }
}

/// Build one step per element of `state`, handing iteration `k` its index,
/// `state[k - 1]` (or `init`) and `state[k]`.
fn unroll<'a, S, C>(state: &'a [S], init: &'a S, mut make: impl FnMut(usize, &'a S, &'a S) -> C) -> Vec<C> {
    let mut steps = Vec::with_capacity(state.len());
    let mut prev = init;
    for (k, cur) in state.iter().enumerate() {
        steps.push(make(k, prev, cur));
        prev = cur;
    }
    steps
}

/// The group a [`RecurrentNode`] or [`RecurrentPipeline`] unrolls into: one
/// step `C` per iteration — a [`Node`] or a [`Pipeline`] respectively.
///
/// Its declared inputs and outputs are *derived* from the children rather than
/// written by hand: inputs are the datasets some iteration reads and none
/// writes (`init` and any loop-invariant inputs), outputs the datasets some
/// iteration writes and none reads (the last state, and any per-iteration
/// outputs nothing downstream in the chain consumes).
pub struct Unrolled<C, N = &'static str>
where
    N: AsRef<str> + Send + Sync,
{
    name: N,
    steps: Vec<C>,
}

impl<C, N> Unrolled<C, N>
where
    N: AsRef<str> + Send + Sync,
{
    /// The per-iteration steps, in iteration order.
    pub fn iterations(&self) -> &[C] {
        &self.steps
    }
}

impl<C, N> StepMeta for Unrolled<C, N>
where
    C: StepMeta,
    N: AsRef<str> + Send + Sync,
{
    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn is_leaf(&self) -> bool {
        false
    }

    fn type_string(&self) -> &'static str {
        "unrolled"
    }

    fn for_each_child<'a>(&'a self, f: &mut dyn FnMut(&'a dyn StepMeta)) {
        for step in &self.steps {
            f(step);
        }
    }

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        let mut produced = HashSet::new();
        for step in &self.steps {
            step.for_each_output(&mut |d| {
                produced.insert(d.id);
            });
        }
        let mut emitted = HashSet::new();
        for step in &self.steps {
            step.for_each_input(&mut |d| {
                if !produced.contains(&d.id) && emitted.insert(d.id) {
                    f(d);
                }
            });
        }
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        let mut consumed = HashSet::new();
        for step in &self.steps {
            step.for_each_input(&mut |d| {
                consumed.insert(d.id);
            });
        }
        let mut emitted = HashSet::new();
        for step in &self.steps {
            step.for_each_output(&mut |d| {
                if !consumed.contains(&d.id) && emitted.insert(d.id) {
                    f(d);
                }
            });
        }
    }
}

impl<C, N, E> Group<E> for Unrolled<C, N>
where
    C: Step<E>,
    N: AsRef<str> + Send + Sync,
{
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn Step<E>)) {
        for step in &self.steps {
            f(step);
        }
    }
}

impl<C, N, E> Step<E> for Unrolled<C, N>
where
    C: Step<E>,
    N: AsRef<str> + Send + Sync,
{
    fn kind(&self) -> StepKind<'_, E> { StepKind::Group(self) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::{Dataset, MemoryDataset, Param};
    use crate::pipeline::StepsMeta;
    use crate::runners::{Runner, SequentialRunner};
    use crate::error::{CheckError, PondError};

    fn names(group: &dyn StepMeta) -> Vec<String> {
        let mut out = Vec::new();
        group.for_each_child(&mut |c| out.push(c.name().to_string()));
        out
    }

    fn ids(group: &dyn StepMeta, outputs: bool) -> Vec<usize> {
        let mut out = Vec::new();
        let mut push = |d: &DatasetRef| out.push(d.id);
        if outputs {
            group.for_each_output(&mut push);
        } else {
            group.for_each_input(&mut push);
        }
        out
    }

    fn id<T>(r: &T) -> usize {
        super::super::traits::ptr_to_id(r)
    }

    #[test]
    fn chain_over_bare_datasets() {
        let init = MemoryDataset::<i32>::new();
        let step = Param(10);
        let state: Vec<MemoryDataset<i32>> = (0..3).map(|_| MemoryDataset::new()).collect();
        init.save(1).unwrap();

        let unrolled = RecurrentNode {
            name: "acc",
            state: &state,
            init: &init,
            input: |prev, _cur| (prev, &step),
            output: |cur| (cur,),
            func: |x: i32, s: i32| (x + s,),
        }
        .build();

        assert_eq!(names(&unrolled), ["0", "1", "2"]);
        // External surface: `init` and the loop-invariant param in; the last
        // slot out. The two intermediate slots stay internal.
        assert_eq!(ids(&unrolled, false), [id(&init), id(&step)]);
        assert_eq!(ids(&unrolled, true), [id(&state[2])]);

        let pipe = (unrolled,);
        pipe.check().unwrap();
        SequentialRunner.run::<PondError>(&pipe, &(), &(), &()).unwrap();
        // Iteration k reads k-1's output; iteration 0 reads `init`.
        let values: Vec<i32> = state.iter().map(|d| d.load().unwrap()).collect();
        assert_eq!(values, [11, 21, 31]);
    }

    struct Slot {
        value: MemoryDataset<i32>,
        step: Param<i32>,
    }

    #[test]
    fn chain_over_sub_catalogs() {
        let init = Slot { value: MemoryDataset::new(), step: Param(0) };
        let state: Vec<Slot> = (1..=3)
            .map(|k| Slot { value: MemoryDataset::new(), step: Param(k * 100) })
            .collect();
        init.value.save(1).unwrap();

        let unrolled = RecurrentNode {
            name: "acc",
            state: &state,
            init: &init,
            input: |prev, cur| (&prev.value, &cur.step),
            output: |cur| (&cur.value,),
            func: |x: i32, s: i32| (x + s,),
        }
        .build();

        assert_eq!(names(&unrolled), ["0", "1", "2"]);
        assert_eq!(
            ids(&unrolled, false),
            [id(&init.value), id(&state[0].step), id(&state[1].step), id(&state[2].step)],
        );
        assert_eq!(ids(&unrolled, true), [id(&state[2].value)]);

        let pipe = (unrolled,);
        pipe.check().unwrap();
        SequentialRunner.run::<PondError>(&pipe, &(), &(), &()).unwrap();
        let values: Vec<i32> = state.iter().map(|s| s.value.load().unwrap()).collect();
        assert_eq!(values, [101, 301, 601]);
    }

    #[test]
    fn empty_state_builds_an_empty_group() {
        let init = MemoryDataset::<i32>::new();
        let state: Vec<MemoryDataset<i32>> = Vec::new();
        let unrolled = RecurrentNode {
            name: "acc",
            state: &state,
            init: &init,
            input: |prev, _cur| (prev,),
            output: |cur| (cur,),
            func: |x: i32| (x,),
        }
        .build();
        assert_eq!(names(&unrolled), Vec::<String>::new());
        (unrolled,).check().unwrap();
    }

    #[test]
    fn check_is_unbounded_under_std() {
        const N: usize = 200;
        let init = MemoryDataset::<i32>::new();
        let state: Vec<MemoryDataset<i32>> = (0..N).map(|_| MemoryDataset::new()).collect();
        let pipe = (RecurrentNode {
            name: "acc",
            state: &state,
            init: &init,
            input: |prev, _cur| (prev,),
            output: |cur| (cur,),
            func: |x: i32| (x + 1,),
        }
        .build(),);

        pipe.check().unwrap();
        // The fixed-capacity path still reports its limit rather than lying.
        assert!(matches!(pipe.check_with_capacity::<20>(), Err(CheckError::CapacityExceeded)));
    }

    /// One iteration's state for the pipeline tests: an intermediate the
    /// iteration's first node writes and its second reads, and the value the
    /// next iteration reads.
    struct Stage {
        acts: MemoryDataset<i32>,
        value: MemoryDataset<i32>,
    }

    impl Stage {
        fn new() -> Self {
            Self { acts: MemoryDataset::new(), value: MemoryDataset::new() }
        }
    }

    #[test]
    fn pipeline_chain_with_intermediates() {
        let init = Stage::new();
        let state: Vec<Stage> = (0..3).map(|_| Stage::new()).collect();
        init.value.save(1).unwrap();

        let unrolled = RecurrentPipeline {
            name: "acc",
            state: &state,
            init: &init,
            input: |prev, _cur| (&prev.value,),
            output: |cur| (&cur.value,),
            pipe: |prev, cur| (
                Node { name: "double", input: (&prev.value,), output: (&cur.acts,), func: |x: i32| (x * 2,) },
                Node { name: "inc", input: (&cur.acts,), output: (&cur.value,), func: |x: i32| (x + 1,) },
            ),
        }
        .build();

        assert_eq!(names(&unrolled), ["0", "1", "2"]);
        assert_eq!(names(&unrolled.iterations()[1]), ["double", "inc"]);
        // The intermediates stay internal to their iteration: only `init` in,
        // only the last value out.
        assert_eq!(ids(&unrolled, false), [id(&init.value)]);
        assert_eq!(ids(&unrolled, true), [id(&state[2].value)]);

        let pipe = (unrolled,);
        pipe.check().unwrap();
        SequentialRunner.run::<PondError>(&pipe, &(), &(), &()).unwrap();
        let values: Vec<i32> = state.iter().map(|s| s.value.load().unwrap()).collect();
        assert_eq!(values, [3, 7, 15]);
    }

    #[test]
    fn pipeline_iterations_are_held_to_their_contract() {
        let init = Stage::new();
        let state: Vec<Stage> = (0..2).map(|_| Stage::new()).collect();

        // `prev.value` is read but not declared.
        let pipe = (RecurrentPipeline {
            name: "acc",
            state: &state,
            init: &init,
            input: |_prev, _cur| (),
            output: |cur| (&cur.value,),
            pipe: |prev, cur| (
                Node { name: "double", input: (&prev.value,), output: (&cur.acts,), func: |x: i32| (x * 2,) },
                Node { name: "inc", input: (&cur.acts,), output: (&cur.value,), func: |x: i32| (x + 1,) },
            ),
        }
        .build(),);

        let err = pipe.check().unwrap_err();
        assert!(
            matches!(err, CheckError::UndeclaredPipelineInput { pipeline_name: "0", .. }),
            "got {err:?}",
        );
    }
}
