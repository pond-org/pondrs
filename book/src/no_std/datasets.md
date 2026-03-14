# no_std Datasets

Four dataset types are available without the standard library.

## `Param<T>`

Read-only parameter values. Works identically in `no_std` and `std`. Requires `T: Clone + Serialize`.

```rust,ignore
let threshold = Param(500u16);
```

See [Param](../datasets/param.md).

## `CellDataset<T>`

Stack-friendly intermediate storage using `Cell`. Requires `T: Copy`.

```rust,ignore
let reading = CellDataset::<u16>::new();
let result = CellDataset::<bool>::new();
```

- `const fn new()` — usable in `static` contexts
- No heap allocation, no locking
- Single-threaded only (`SequentialRunner`)

See [Cell Dataset](../datasets/cell.md).

## `RegisterDataset<T>`

Volatile memory-mapped register access. Reads and writes at a raw memory address using `read_volatile` / `write_volatile`.

```rust,ignore
// SAFETY: address must point to a valid, aligned register
static SENSOR: RegisterDataset<u16> = unsafe { RegisterDataset::new(0x4000_0000) };
```

- `T` is typically `u8`, `u16`, or `u32`
- `const unsafe fn new(address: usize)` — suitable for `static` declarations
- `load()` reads the register, `save()` writes it

## `GpioDataset`

A single GPIO pin within a memory-mapped register. Reads and writes a single bit at a given position.

```rust,ignore
// SAFETY: address must point to a valid GPIO port register
static LED: GpioDataset = unsafe { GpioDataset::new(0x4002_0000, 5, "LED1") };
```

- `load()` returns `bool` — whether the bit is set
- `save(true)` sets the bit, `save(false)` clears it
- Preserves other bits in the register (read-modify-write)
- The `label` field is used in visualization

## Hardware dataset safety

Both `RegisterDataset` and `GpioDataset` use `unsafe` constructors because they access raw memory addresses. They implement `Send + Sync` via unsafe impls — this is safe in single-threaded embedded contexts but the caller is responsible for preventing data races in multi-threaded environments.

## Example: embedded pipeline

```rust,ignore
struct Catalog {
    sensor: RegisterDataset<u16>,
    reading: CellDataset<u16>,
    alert: GpioDataset,
}

struct Params {
    threshold: Param<u16>,
}

fn pipeline<'a>(cat: &'a Catalog, params: &'a Params) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "read",
            func: |raw: u16| (raw,),
            input: (&cat.sensor,),
            output: (&cat.reading,),
        },
        Node {
            name: "check",
            func: |value: u16, thresh: u16| (value > thresh,),
            input: (&cat.reading, &params.threshold),
            output: (&cat.alert,),
        },
    )
}
```
