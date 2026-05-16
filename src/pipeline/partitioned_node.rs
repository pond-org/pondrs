use std::prelude::v1::*;
use std::collections::HashMap;

use core::marker::PhantomData;
use serde::{Serialize, de::DeserializeOwned};

use crate::datasets::{FileDataset, FromThunk, IntoThunk, PartitionedDataset, Thunk};
use crate::error::PondError;

use super::into_result::IntoNodeResult;
use super::node::CompatibleOutput;
use super::stable::StableFn;
use super::traits::{DatasetEvent, DatasetRef, NodeInput, NodeOutput, StepInfo, RunnableStep};

pub struct PartitionedNode<'a, F, D1, D2, T1, T2>
where
    D1: FileDataset + Serialize + DeserializeOwned,
    D2: FileDataset + Serialize + DeserializeOwned,
    F: StableFn<(T1,)>,
    F::Output: CompatibleOutput<(T2,)>,
{
    pub name: &'static str,
    pub func: F,
    pub input: &'a PartitionedDataset<D1>,
    pub output: &'a PartitionedDataset<D2>,
    pub _marker: PhantomData<(T1, T2)>,
}

impl<'a, F, D1, D2, T1, T2> PartitionedNode<'a, F, D1, D2, T1, T2>
where
    D1: FileDataset + Serialize + DeserializeOwned,
    D2: FileDataset + Serialize + DeserializeOwned,
    F: StableFn<(T1,)>,
    F::Output: CompatibleOutput<(T2,)>,
{
    pub fn new(
        name: &'static str,
        func: F,
        input: &'a PartitionedDataset<D1>,
        output: &'a PartitionedDataset<D2>,
    ) -> Self {
        Self { name, func, input, output, _marker: PhantomData }
    }
}

impl<F, D1, D2, T1, T2> StepInfo for PartitionedNode<'_, F, D1, D2, T1, T2>
where
    D1: FileDataset + Serialize + DeserializeOwned + Send + Sync + 'static,
    D2: FileDataset + Serialize + DeserializeOwned + Send + Sync + 'static,
    D1::SaveItem: Send,
    D2::SaveItem: Send,
    D1::Error: Send,
    D2::Error: Send,
    PondError: From<D1::Error> + From<D2::Error>,
    F: StableFn<(T1,)> + Send + Sync,
    F::Output: CompatibleOutput<(T2,)>,
    T1: Send + Sync,
    T2: Send + Sync,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn type_string(&self) -> &'static str {
        core::any::type_name::<F>()
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn StepInfo)) {}

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.input));
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        f(&DatasetRef::from_ref(self.output));
    }
}

impl<F, D1, D2, T1, T2, E> RunnableStep<E> for PartitionedNode<'_, F, D1, D2, T1, T2>
where
    D1: FileDataset + Serialize + DeserializeOwned + Send + Sync + 'static,
    D2: FileDataset + Serialize + DeserializeOwned + Send + Sync + 'static,
    D1::LoadItem: IntoThunk<T1> + Send + 'static,
    D1::SaveItem: Send,
    D2::SaveItem: FromThunk<T2> + Send,
    PondError: From<D1::Error> + From<D2::Error>,
    D1::Error: Send,
    D2::Error: Send,
    T1: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    F: StableFn<(T1,)> + Clone + Send + Sync + 'static,
    F::Output: IntoNodeResult<(T2,), PondError> + CompatibleOutput<(T2,)>,
    E: From<PondError>,
{
    fn call(&self, on_event: &mut dyn FnMut(&DatasetRef<'_>, DatasetEvent<'_>)) -> Result<(), E> {
        let (input_map,) = (self.input,).load_data(on_event).map_err(E::from)?;

        let output_map = input_map
            .into_iter()
            .map(|(key, elem)| {
                let func = self.func.clone();
                let in_thunk: Thunk<T1> = elem.into_thunk();
                let out_thunk: Thunk<T2> = Box::new(move || {
                    let value = in_thunk()?;
                    let (result,) = StableFn::call(&func, (value,)).into_node_result()?;
                    Ok(result)
                });
                let save_item = D2::SaveItem::from_thunk(out_thunk)?;
                Ok((key, save_item))
            })
            .collect::<Result<HashMap<_, _>, PondError>>()
            .map_err(E::from)?;

        (self.output,).save_data((output_map,), on_event).map_err(E::from)?;
        Ok(())
    }

    fn for_each_child_step<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn RunnableStep<E>)) {}
    fn as_pipeline_info(&self) -> &dyn StepInfo { self }
}
