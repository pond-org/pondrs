# Custom Datasets

You can implement the `Dataset` trait for any type to integrate custom data sources into your pipeline.

## The `Dataset` trait

```rust,ignore
pub trait Dataset: serde::Serialize {
    type LoadItem;
    type SaveItem;
    type Error;

    fn load(&self) -> Result<Self::LoadItem, Self::Error>;
    fn save(&self, output: Self::SaveItem) -> Result<(), Self::Error>;
    fn is_param(&self) -> bool { false }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> { None }
}
```

## Example: a plain text dataset

```rust,ignore
#[derive(Serialize, Deserialize, Clone)]
pub struct TextDataset {
    path: String,
}

impl Dataset for TextDataset {
    type LoadItem = String;
    type SaveItem = String;
    type Error = PondError;

    fn load(&self) -> Result<String, PondError> {
        Ok(std::fs::read_to_string(&self.path)?)
    }

    fn save(&self, text: String) -> Result<(), PondError> {
        std::fs::write(&self.path, text)?;
        Ok(())
    }
}
```

## Checklist

1. **Derive `Serialize`** (required by the supertrait) — and usually `Deserialize` too, so the catalog can be loaded from YAML.

2. **Name the type with a `Dataset` suffix** — this is a *requirement*, not a style preference. See [Naming](#naming-is-a-requirement) below.

3. **Choose your error type** — use `PondError` for simplicity, or a custom error type if you want to preserve error detail (see [Dataset Errors](../error_handling/datasets.md)).

4. **Implement `html()`** (optional, `std` only) — return an HTML snippet for the viz dashboard. This is shown in the dataset detail panel.

## Naming is a requirement

A custom dataset type's ident **must end in `Dataset`**, and a catalog or
param-group struct **must not**.

Datasets are identified by the address of their catalog field, and in Rust a
struct shares its address with its first field. The catalog indexer walks the
catalog with a serde `Serializer` to build its `address -> name` map, and it has
only the serde struct name to tell "this is the dataset" from "this is a
container holding one". It stops recursing at:

- a type whose ident ends in `Dataset`
- the type `Param`

Everything else it recurses into. So a dataset named `S3Store` is mistaken for a
container: the indexer descends into it, and the name of its first field wins.

```rust,ignore
#[derive(Serialize)]
struct S3Store {          // ✗ does not end in `Dataset`
    bucket: String,       //   &store == &store.bucket
}

struct Catalog {
    s3: S3Store,
}
```

The dataset now resolves to `s3.bucket` instead of `s3` everywhere a name is
shown: log lines, hook callbacks, the viz graph, the cache hook's keys. Nothing
fails — the pipeline runs exactly as written — but every name is wrong, and
nothing tells you. Renaming the type to `S3StoreDataset` fixes it.

`my_app check` reports both halves of this: `CheckWarning::UnconventionalDatasetType`
from the pipeline walk alone, and `CatalogWarning::MisresolvedName` from the
catalog cross-check, which also names the correct name it should have had. See
[Check](../pipelines/check.md#warnings).

Two more identity rules follow from the same address-is-identity scheme:

- **Don't make a dataset type zero-sized.** A zero-sized field shares its address
  with whatever follows it, so two of them collapse into one node in the DAG.
  `check` reports this as `CheckError::AliasedDatasets`, and a single one as
  `CheckWarning::ZeroSizedDataset`. Give the type a real field.
- **Keep datasets reachable by the serde walk.** A dataset behind
  `#[serde(skip)]`, inside a tuple (the indexer does not descend into tuples;
  `Vec` elements *are* named, by index), or behind a hand-written `Serialize`
  that does not pass
  `&self.field` through, is never named at all —
  `CatalogWarning::UnnamedDataset`.

## The `FileDataset` trait

If your dataset is backed by a file, implement `FileDataset` to enable use with `PartitionedDataset`:

```rust,ignore
pub trait FileDataset: Dataset + Clone {
    fn path(&self) -> &str;
    fn set_path(&mut self, path: &str);
}
```

This lets `PartitionedDataset` clone your dataset template and point each partition at a different file.

