# no_std Pipelines

pondrs is a `#![no_std]` crate at its core. The `std` feature adds filesystem I/O, threading, logging, CLI parsing, and additional dataset types — but the fundamental pipeline model works without any of it.

## What's available without `std`

| Component | no_std | std |
|-----------|--------|-----|
| `Node`, `Pipeline`, `Steps`, `StepInfo` | yes | yes |
| `PipelineInfo`, `RunnableStep` | yes | yes |
| `check()` validation | yes | yes |
| `Param<T>` | yes | yes |
| `CellDataset<T>` | yes | yes |
| `RegisterDataset<T>` | yes | yes |
| `GpioDataset` | yes | yes |
| `SequentialRunner` | yes | yes |
| `Hook` / `Hooks` traits | yes | yes |
| `App::new()` | yes | yes |
| `PondError` (limited variants) | yes | yes |
| `MemoryDataset<T>` | — | yes |
| `ParallelRunner` | — | yes |
| `LoggingHook` | — | yes |
| CLI parsing / YAML loading | — | yes |
| Catalog indexer (dataset names) | — | yes |
| File-backed datasets | — | yes |

## Building for no_std

```sh
cargo build --no-default-features --lib
```

No allocator is required. All dataset tracking in `check()` uses fixed-size stack arrays.

## Typical embedded pattern

```rust,ignore
use pondrs::datasets::{CellDataset, Param, RegisterDataset, GpioDataset};
use pondrs::error::PondError;
use pondrs::{App, Node, Steps};

static SENSOR: RegisterDataset<u16> = unsafe { RegisterDataset::new(0x4000_0000) };
static LED: GpioDataset = unsafe { GpioDataset::new(0x4002_0000, 5, "LED") };

fn pipeline<'a>(
    cat: &'a Catalog,
    params: &'a Params,
) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "read_sensor",
            func: |raw: u16| (raw,),
            input: (&cat.sensor,),
            output: (&cat.reading,),
        },
        Node {
            name: "check_threshold",
            func: |value: u16, threshold: u16| (value > threshold,),
            input: (&cat.reading, &params.threshold),
            output: (&cat.led,),
        },
    )
}
```

See [Datasets](./datasets.md) for details on the no_std dataset types and [App & Debugging](./app.md) for the no_std `App` pattern.
