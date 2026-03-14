# Runners

Runners execute pipeline steps. pondrs provides two built-in runners and lets you select between them at runtime.

## The `Runner` trait

```rust,ignore
pub trait Runner {
    fn name(&self) -> &'static str;

    fn run<E>(
        &self,
        pipe: &impl Steps<E>,
        catalog: &impl Serialize,
        params: &impl Serialize,
        hooks: &impl Hooks,
    ) -> Result<(), E>
    where
        E: From<PondError> + Send + Sync + Display + Debug + 'static;
}
```

- **`name()`** — identifies the runner for CLI selection (`--runner sequential`)
- **`run()`** — executes the pipeline, calling hooks at each lifecycle point

## The `Runners` trait

Multiple runners compose as tuples, enabling runtime selection:

```rust,ignore
pub trait Runners {
    fn first_name(&self) -> &'static str;
    fn run_by_name<E>(&self, name: &str, ...) -> Option<Result<(), E>>;
    fn for_each_name(&self, f: &mut dyn FnMut(&str));
}
```

The default runners depend on the feature set:

- **`std`** — `(SequentialRunner, ParallelRunner)` — sequential is the default
- **`no_std`** — `(SequentialRunner,)` — only sequential is available

## Selecting a runner

Via CLI:

```sh
$ my_app run                       # uses default (sequential)
$ my_app run --runner parallel     # uses parallel runner
$ my_app run --runner sequential   # explicit sequential
```

Via code:

```rust,ignore
App::new(catalog, params)
    .with_runners((SequentialRunner, ParallelRunner))
    .execute(pipeline)?;
```

## Custom runners

You can implement `Runner` for your own types. Add them to the runners tuple:

```rust,ignore
App::new(catalog, params)
    .with_runners((SequentialRunner, ParallelRunner, MyDistributedRunner))
    .execute(pipeline)?;
```

```sh
$ my_app run --runner my_distributed
```

This chapter covers:

- **[Runner Trait](./runner_trait.md)** — implementing a custom runner
- **[Sequential Runner](./sequential.md)** — runs nodes in definition order
- **[Parallel Runner](./parallel.md)** — runs independent nodes concurrently
