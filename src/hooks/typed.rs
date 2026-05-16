use core::marker::PhantomData;

use crate::pipeline::{DatasetRef, StepInfo};

use super::Hook;

pub trait TypedHook<T: 'static>: Sync {
    fn after_load(&self, _n: &dyn StepInfo, _ds: &DatasetRef, _value: &T) {}
    fn before_save(&self, _n: &dyn StepInfo, _ds: &DatasetRef, _value: &T) {}
}

pub struct TypedHookAdapter<T, H>(H, PhantomData<fn() -> T>);

impl<T: 'static, H: TypedHook<T> + Sync> Hook for TypedHookAdapter<T, H> {
    fn after_dataset_loaded(&self, n: &dyn StepInfo, ds: &DatasetRef, value: &dyn core::any::Any) {
        if let Some(v) = value.downcast_ref::<T>() {
            self.0.after_load(n, ds, v);
        }
    }

    fn before_dataset_saved(&self, n: &dyn StepInfo, ds: &DatasetRef, value: &dyn core::any::Any) {
        if let Some(v) = value.downcast_ref::<T>() {
            self.0.before_save(n, ds, v);
        }
    }
}

pub trait IntoTypedHook<T: 'static>: TypedHook<T> + Sized + Sync {
    fn typed(self) -> TypedHookAdapter<T, Self> {
        TypedHookAdapter(self, PhantomData)
    }
}

impl<T: 'static, H: TypedHook<T> + Sync> IntoTypedHook<T> for H {}
