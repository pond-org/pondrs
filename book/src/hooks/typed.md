# Typed Hooks

The `Hook` trait passes dataset values as `&dyn Any`, requiring manual downcasting. `TypedHook<T>` provides type-safe access — you implement methods that receive `&T` directly, and non-matching types are silently ignored.

## The `TypedHook<T>` trait

```rust,ignore
pub trait TypedHook<T: 'static>: Sync {
    fn after_load(&self, _n: &dyn StepInfo, _ds: &DatasetRef, _value: &T) -> Result<(), HookAbort> {
        Ok(())
    }
    fn before_save(&self, _n: &dyn StepInfo, _ds: &DatasetRef, _value: &T) -> Result<HookControl, HookAbort> {
        Ok(HookControl::Continue)
    }
}
```

- `after_load` fires after a dataset of type `T` is loaded (corresponds to `Hook::after_dataset_loaded`)
- `before_save` fires before a dataset of type `T` is saved (corresponds to `Hook::before_dataset_saved`)

Datasets of other types are silently ignored — the adapter checks the type at runtime and skips non-matching values.

## Using typed hooks

A `TypedHook<T>` cannot be used directly in a hooks tuple because the tuple expects `Hook` impls. Call `.typed()` to wrap it in a `TypedHookAdapter` that implements `Hook`:

```rust,ignore
let hooks = (MyTypedHook.typed(),);
```

The `.typed()` method is provided by `IntoTypedHook<T>`, which has a blanket implementation for all `TypedHook<T> + Sized + Sync` types.

## Example: recording loaded values

```rust,ignore
use std::sync::{Arc, Mutex};

struct I32Recorder(Arc<Mutex<Vec<i32>>>);

impl TypedHook<i32> for I32Recorder {
    fn after_load(&self, _n: &dyn StepInfo, _ds: &DatasetRef, value: &i32) -> Result<(), HookAbort> {
        self.0.lock().unwrap().push(*value);
        Ok(())
    }
}
```

Register it with `.typed()`:

```rust,ignore
let recorded = Arc::new(Mutex::new(Vec::new()));

App::new(catalog, params)
    .with_hooks((I32Recorder(Arc::clone(&recorded)).typed(),))
    .execute(pipeline)?;

// recorded now contains all i32 values that were loaded during the pipeline run
// String, f64, and other dataset types were silently ignored
```

## Example: validation before save

```rust,ignore
struct RejectNegative;

impl TypedHook<i32> for RejectNegative {
    fn before_save(&self, _n: &dyn StepInfo, _ds: &DatasetRef, value: &i32) -> Result<HookControl, HookAbort> {
        if *value < 0 {
            Err(HookAbort("negative value rejected"))
        } else {
            Ok(HookControl::Continue)
        }
    }
}
```

If any node tries to save a negative `i32`, the pipeline stops with `PondError::HookAbort("negative value rejected")`. Saves of other types (e.g. `String`) pass through unaffected.

## Combining with other hooks

Typed hooks compose with regular hooks in the same tuple:

```rust,ignore
.with_hooks((
    LoggingHook::new(),
    RejectNegative.typed(),
))
```

You can also have multiple typed hooks for different types:

```rust,ignore
.with_hooks((
    I32Recorder(state).typed(),
    StringValidator.typed(),
))
```
