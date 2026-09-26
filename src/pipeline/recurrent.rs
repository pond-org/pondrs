//! `RecurrentNode` — a loop unrolled into one real [`Node`] per iteration.

use std::collections::HashSet;
use std::prelude::v1::*;

use crate::error::PondError;

use super::into_result::IntoNodeResult;
use super::node::{CompatibleOutput, Node};
use super::stable::StableFn;
use super::traits::{DatasetRef, Group, NodeInput, NodeInputMeta, NodeOutput, NodeOutputMeta, Step, StepKind, StepMeta};

/// A computation repeated once per element of a state vector, each iteration
/// reading the previous iteration's state and writing its own.
///
/// [`build`](Self::build) unrolls it into an [`Unrolled`] group holding one
/// plain [`Node`] per iteration, named `"{name}/{k}"`. Because the children are
/// real nodes, everything that works on nodes works on each iteration
/// individually: `--from-nodes train/47` resumes mid-chain, `CacheHook` keys
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
    /// Base name; iteration `k` is named `"{name}/{k}"`.
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
    pub fn build(self) -> Unrolled<F, In, Out, N> {
        let base = self.name.as_ref();
        let mut nodes = Vec::with_capacity(self.state.len());
        let mut prev: &'a S = self.init;
        for (k, cur) in self.state.iter().enumerate() {
            nodes.push(Node {
                name: format!("{base}/{k}"),
                input: (self.input)(prev, cur),
                output: (self.output)(cur),
                func: self.func.clone(),
            });
            prev = cur;
        }
        Unrolled { name: self.name, nodes }
    }
}

/// The group a [`RecurrentNode`] unrolls into: one [`Node`] per iteration.
///
/// Its declared inputs and outputs are *derived* from the children rather than
/// written by hand: inputs are the datasets some iteration reads and none
/// writes (`init` and any loop-invariant inputs), outputs the datasets some
/// iteration writes and none reads (the last state, and any per-iteration
/// outputs nothing downstream in the chain consumes).
pub struct Unrolled<F, In, Out, N = &'static str>
where
    In: NodeInputMeta,
    Out: NodeOutputMeta,
    F: StableFn<In::Args>,
    F::Output: CompatibleOutput<Out::Output>,
    N: AsRef<str> + Send + Sync,
{
    name: N,
    nodes: Vec<Node<F, In, Out, String>>,
}

impl<F, In, Out, N> Unrolled<F, In, Out, N>
where
    In: NodeInputMeta,
    Out: NodeOutputMeta,
    F: StableFn<In::Args>,
    F::Output: CompatibleOutput<Out::Output>,
    N: AsRef<str> + Send + Sync,
{
    /// The per-iteration nodes, in iteration order.
    pub fn nodes(&self) -> &[Node<F, In, Out, String>] {
        &self.nodes
    }
}

impl<F, In, Out, N> StepMeta for Unrolled<F, In, Out, N>
where
    In: NodeInputMeta + Send + Sync,
    Out: NodeOutputMeta + Send + Sync,
    F: StableFn<In::Args> + Send + Sync,
    F::Output: CompatibleOutput<Out::Output>,
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
        for node in &self.nodes {
            f(node);
        }
    }

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        let mut produced = HashSet::new();
        for node in &self.nodes {
            node.for_each_output(&mut |d| {
                produced.insert(d.id);
            });
        }
        let mut emitted = HashSet::new();
        for node in &self.nodes {
            node.for_each_input(&mut |d| {
                if !produced.contains(&d.id) && emitted.insert(d.id) {
                    f(d);
                }
            });
        }
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        let mut consumed = HashSet::new();
        for node in &self.nodes {
            node.for_each_input(&mut |d| {
                consumed.insert(d.id);
            });
        }
        let mut emitted = HashSet::new();
        for node in &self.nodes {
            node.for_each_output(&mut |d| {
                if !consumed.contains(&d.id) && emitted.insert(d.id) {
                    f(d);
                }
            });
        }
    }
}

impl<F, In, Out, N, E, R> Group<E> for Unrolled<F, In, Out, N>
where
    In: NodeInput<E> + Send + Sync,
    Out: NodeOutput<E> + Send + Sync,
    F: StableFn<In::Args, Output = R> + Send + Sync,
    R: IntoNodeResult<Out::Output, E>,
    E: From<PondError>,
    N: AsRef<str> + Send + Sync,
{
    fn for_each_child_step<'a>(&'a self, f: &mut dyn FnMut(&'a dyn Step<E>)) {
        for node in &self.nodes {
            f(node);
        }
    }
}

impl<F, In, Out, N, E, R> Step<E> for Unrolled<F, In, Out, N>
where
    In: NodeInput<E> + Send + Sync,
    Out: NodeOutput<E> + Send + Sync,
    F: StableFn<In::Args, Output = R> + Send + Sync,
    R: IntoNodeResult<Out::Output, E>,
    E: From<PondError>,
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
    use crate::error::CheckError;

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

        assert_eq!(names(&unrolled), ["acc/0", "acc/1", "acc/2"]);
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

        assert_eq!(names(&unrolled), ["acc/0", "acc/1", "acc/2"]);
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
        assert!(names(&unrolled).is_empty());
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
}
