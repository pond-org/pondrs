//! Prototype: a fully dynamic, YAML-defined catalog built entirely on the
//! existing pondrs API — no changes to the library.
//!
//! This explores what a Python-facing `DynCatalog` would look like. The catalog
//! shape (field names + dataset types) comes from YAML rather than a Rust
//! struct, and pipelines can be assembled at runtime from names alone.
//!
//! Two things are demonstrated:
//!
//! 1. `AnyDataset` — a sum type over the built-in dataset types that itself
//!    implements `Dataset`, so `&AnyDataset` is a valid node input/output via
//!    the existing blanket impls in `pipeline::traits`.
//! 2. `DynNode` — a runtime-arity node implementing `StepInfo` / `LeafStep` /
//!    `RunnableStep` by hand, boxed into a `StepVec`. This is the shape a
//!    Python-defined node would take.
//!
//! Usage:
//!   cargo run --example dyn_catalog

use std::collections::HashMap;

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Value as JsonValue, json};

use pondrs::datasets::{JsonDataset, TextDataset};
use pondrs::error::PondError;
use pondrs::hooks::{HookAbort, HookControl};
use pondrs::{
    Dataset, DatasetEvent, DatasetRef, LeafStep, Node, PipelineInfo, RunnableStep, SequentialRunner,
    StepInfo, StepKind, StepVec,
};
use pondrs::runners::Runner;

// ─── DynValue: the erased load/save type ──────────────────────────────────────

/// The single `LoadItem`/`SaveItem` type shared by every dynamic dataset.
///
/// In the Python binding this is where `pyo3` conversion would hook in; here it
/// is a plain enum so the prototype stays dependency-free.
#[derive(Debug, Clone)]
pub enum DynValue {
    Text(String),
    Json(JsonValue),
}

impl DynValue {
    fn kind(&self) -> &'static str {
        match self {
            DynValue::Text(_) => "text",
            DynValue::Json(_) => "json",
        }
    }

    /// Typed accessor. Conversion failures are runtime errors — the price of a
    /// dynamic catalog, and unavoidable once the shape comes from YAML.
    pub fn as_text(&self) -> Result<&str, PondError> {
        match self {
            DynValue::Text(s) => Ok(s),
            other => Err(PondError::Custom(format!(
                "expected text value, got {}",
                other.kind()
            ))),
        }
    }

    pub fn as_json(&self) -> Result<&JsonValue, PondError> {
        match self {
            DynValue::Json(v) => Ok(v),
            other => Err(PondError::Custom(format!(
                "expected json value, got {}",
                other.kind()
            ))),
        }
    }
}

// ─── AnyDataset: one variant per built-in dataset type ────────────────────────

/// Generates the `AnyDataset` sum type plus its `Dataset` impl.
///
/// Adding a dataset type to the dynamic catalog is one line. This is what keeps
/// "one variant per built-in" from being a maintenance problem: the delegation
/// is mechanical, exactly like `CacheDataset`/`LazyDataset` already do for
/// `html()`.
macro_rules! any_dataset {
    ($( $variant:ident($ty:ty) <=> $val:ident ),* $(,)?) => {
        /// Tagged union over the dataset types available to a dynamic catalog.
        ///
        /// Deserializes from `{ type: Text, path: ... }`.
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(tag = "type")]
        pub enum AnyDataset {
            $( $variant($ty), )*
        }

        impl AnyDataset {
            fn kind(&self) -> &'static str {
                match self { $( AnyDataset::$variant(_) => stringify!($variant), )* }
            }
        }

        impl Dataset for AnyDataset {
            type LoadItem = DynValue;
            type SaveItem = DynValue;
            type Error = PondError;

            fn load(&self) -> Result<DynValue, PondError> {
                match self {
                    $( AnyDataset::$variant(d) => Ok(DynValue::$val(d.load()?)), )*
                }
            }

            fn save(&self, value: DynValue) -> Result<(), PondError> {
                match (self, value) {
                    $( (AnyDataset::$variant(d), DynValue::$val(v)) => d.save(v), )*
                    (ds, v) => Err(PondError::Custom(format!(
                        "cannot save a {} value into a {} dataset",
                        v.kind(), ds.kind(),
                    ))),
                }
            }

            // Metadata delegates variant-wise — same shape as the existing
            // wrapper datasets, so viz/caching/params work unchanged.
            fn is_param(&self) -> bool {
                match self { $( AnyDataset::$variant(d) => Dataset::is_param(d), )* }
            }
            fn content_hash(&self) -> Option<u64> {
                match self { $( AnyDataset::$variant(d) => Dataset::content_hash(d), )* }
            }
            fn is_persistent(&self) -> bool {
                match self { $( AnyDataset::$variant(d) => Dataset::is_persistent(d), )* }
            }
            fn html(&self) -> Option<String> {
                match self { $( AnyDataset::$variant(d) => Dataset::html(d), )* }
            }
        }
    };
}

any_dataset! {
    Text(TextDataset) <=> Text,
    Json(JsonDataset) <=> Json,
}

// ─── DynCatalog ───────────────────────────────────────────────────────────────

/// A catalog whose shape is defined by YAML rather than a Rust struct.
///
/// Entries are boxed so their addresses stay stable regardless of map
/// rehashing — `ptr_to_id` identity is what the whole graph is keyed on.
pub struct DynCatalog {
    names: Vec<String>,
    items: HashMap<String, Box<AnyDataset>>,
}

impl DynCatalog {
    pub fn from_yaml_str(source: &str) -> Result<Self, PondError> {
        let raw: serde_yaml::Mapping = serde_yaml::from_str(source)?;
        let mut names = Vec::with_capacity(raw.len());
        let mut items = HashMap::with_capacity(raw.len());

        for (key, value) in raw {
            let name = key
                .as_str()
                .ok_or_else(|| PondError::Custom("catalog keys must be strings".into()))?
                .to_string();
            let ds: AnyDataset = serde_yaml::from_value(value)
                .map_err(|e| PondError::Custom(format!("dataset `{name}`: {e}")))?;
            names.push(name.clone());
            items.insert(name, Box::new(ds));
        }

        Ok(Self { names, items })
    }

    /// Look up a dataset by name. Unknown names produce a suggestion, since a
    /// dynamic catalog has no compiler to catch typos.
    pub fn get(&self, name: &str) -> Result<&AnyDataset, PondError> {
        match self.items.get(name) {
            Some(ds) => Ok(&**ds),
            None => Err(PondError::Custom(format!(
                "unknown dataset `{name}`{}",
                match self.closest(name) {
                    Some(c) => format!(" — did you mean `{c}`?"),
                    None => String::new(),
                }
            ))),
        }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    fn closest(&self, name: &str) -> Option<&str> {
        self.names
            .iter()
            .map(|n| (edit_distance(n, name), n.as_str()))
            .filter(|(d, _)| *d <= 3)
            .min_by_key(|(d, _)| *d)
            .map(|(_, n)| n)
    }
}

/// Serialized as a map of name → dataset so the existing catalog indexer
/// recurses into entries and records `ptr_to_id(&AnyDataset)` under the map
/// key — the same address `get()` hands to nodes. This is exactly the trick
/// `TemplatedCatalog` already uses.
impl Serialize for DynCatalog {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.names.len()))?;
        for name in &self.names {
            let ds: &AnyDataset = &*self.items[name];
            map.serialize_entry(name, ds)?;
        }
        map.end()
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        core::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

// ─── DynNode: a runtime-arity node ────────────────────────────────────────────

/// A node whose input/output arity is known only at runtime.
///
/// This is the shape a Python-defined node takes: `func` would be a
/// `Py<PyAny>` and the conversion would run through `DynValue`. Everything
/// else — hooks, graph building, `check()`, both runners — is inherited from
/// the traits implemented below, with no library changes.
pub struct DynNode<'a> {
    pub name: &'static str,
    pub inputs: Vec<&'a AnyDataset>,
    pub outputs: Vec<&'a AnyDataset>,
    #[allow(clippy::type_complexity)]
    pub func: Box<dyn Fn(Vec<DynValue>) -> Result<Vec<DynValue>, PondError> + Send + Sync + 'a>,
}

impl StepInfo for DynNode<'_> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_leaf(&self) -> bool {
        true
    }

    fn type_string(&self) -> &'static str {
        "DynNode"
    }

    fn for_each_child<'a>(&'a self, _f: &mut dyn FnMut(&'a dyn StepInfo)) {}

    fn for_each_input<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for ds in &self.inputs {
            f(&DatasetRef::from_ref(*ds));
        }
    }

    fn for_each_output<'s>(&'s self, f: &mut dyn FnMut(&DatasetRef<'s>)) {
        for ds in &self.outputs {
            f(&DatasetRef::from_ref(*ds));
        }
    }
}

impl<E: From<PondError>> LeafStep<E> for DynNode<'_> {
    fn call(
        &self,
        on_event: &mut dyn FnMut(
            &DatasetRef<'_>,
            DatasetEvent<'_>,
        ) -> Result<HookControl, HookAbort>,
    ) -> Result<(), E> {
        let mut args = Vec::with_capacity(self.inputs.len());
        for ds in &self.inputs {
            let r = DatasetRef::from_ref(*ds);
            on_event(&r, DatasetEvent::BeforeLoad).map_err(PondError::from)?;
            let value = ds.load()?;
            on_event(&r, DatasetEvent::AfterLoad(&value)).map_err(PondError::from)?;
            args.push(value);
        }

        let produced = (self.func)(args)?;
        if produced.len() != self.outputs.len() {
            return Err(E::from(PondError::Custom(format!(
                "node `{}` returned {} values but declares {} outputs",
                self.name,
                produced.len(),
                self.outputs.len()
            ))));
        }

        for (ds, value) in self.outputs.iter().zip(produced) {
            let r = DatasetRef::from_ref(*ds);
            let control = on_event(&r, DatasetEvent::BeforeSave(&value))
                .map_err(PondError::from)?;
            if control != HookControl::Skip {
                ds.save(value)?;
                on_event(&r, DatasetEvent::AfterSave).map_err(PondError::from)?;
            }
        }
        Ok(())
    }
}

impl<E: From<PondError>> RunnableStep<E> for DynNode<'_> {
    fn kind(&self) -> StepKind<'_, E> {
        StepKind::Leaf(self)
    }
    fn as_pipeline_info(&self) -> &dyn StepInfo {
        self
    }
}

// ─── Demo ─────────────────────────────────────────────────────────────────────

const CATALOG_YAML: &str = "
greeting:
  type: Text
  path: /tmp/pondrs-dyn/greeting.txt
shout:
  type: Text
  path: /tmp/pondrs-dyn/shout.txt
report:
  type: Json
  path: /tmp/pondrs-dyn/report.json
";

fn main() -> Result<(), PondError> {
    std::fs::create_dir_all("/tmp/pondrs-dyn")?;
    std::fs::write("/tmp/pondrs-dyn/greeting.txt", "hello from yaml")?;

    let cat = DynCatalog::from_yaml_str(CATALOG_YAML)?;
    println!("catalog entries: {:?}\n", cat.names());

    // ── 1. A statically-shaped node over a dynamic catalog ───────────────────
    // `&AnyDataset` is a valid node input/output through the existing blanket
    // impls, so ordinary `Node` works with no changes at all.
    let static_pipe = (Node {
        name: "shout",
        input: (cat.get("greeting")?,),
        output: (cat.get("shout")?,),
        func: |v: DynValue| -> Result<(DynValue,), PondError> {
            Ok((DynValue::Text(v.as_text()?.to_uppercase()),))
        },
    },);

    static_pipe.check().map_err(|e| PondError::Custom(format!("{e:?}")))?;
    SequentialRunner.run::<PondError>(&static_pipe, &cat, &(), &())?;
    println!("shout.txt   = {:?}", std::fs::read_to_string("/tmp/pondrs-dyn/shout.txt")?);

    // ── 2. Runtime-arity nodes assembled from names ──────────────────────────
    // This is the Python path: names and callables only, resolved at build time.
    let dyn_pipe: StepVec<PondError> = vec![
        DynNode {
            name: "summarize",
            inputs: vec![cat.get("greeting")?, cat.get("shout")?],
            outputs: vec![cat.get("report")?],
            func: Box::new(|args| {
                let plain = args[0].as_text()?;
                let loud = args[1].as_text()?;
                Ok(vec![DynValue::Json(json!({
                    "original": plain,
                    "shouted": loud,
                    "length": plain.len(),
                }))])
            }),
        }
        .boxed(),
    ];

    dyn_pipe.check().map_err(|e| PondError::Custom(format!("{e:?}")))?;
    SequentialRunner.run(&dyn_pipe, &cat, &(), &())?;
    println!("report.json = {}", std::fs::read_to_string("/tmp/pondrs-dyn/report.json")?);

    // ── 3. Names resolve through the existing catalog indexer ────────────────
    let index = pondrs::index_catalog(&cat);
    for name in cat.names() {
        let ds = cat.get(name)?;
        let id = pondrs::DatasetRef::from_ref(ds).id;
        println!("indexed {:>9} -> {:?}", name, index.get(id));
    }

    // ── 4. Unknown names produce a suggestion, not a panic ───────────────────
    match cat.get("reprot") {
        Err(e) => println!("\nlookup error: {e}"),
        Ok(_) => unreachable!(),
    }

    // ── 5. `check()` still sees real dependencies ────────────────────────────
    // Proof that pointer identity survives the dynamic path: a node reading a
    // dataset produced by a *later* node is rejected, exactly as for `Node`.
    let bad: StepVec<PondError> = vec![
        DynNode {
            name: "reads_too_early",
            inputs: vec![cat.get("report")?],
            outputs: vec![cat.get("shout")?],
            func: Box::new(|_| Ok(vec![DynValue::Text(String::new())])),
        }
        .boxed(),
        DynNode {
            name: "produces_report",
            inputs: vec![cat.get("greeting")?],
            outputs: vec![cat.get("report")?],
            func: Box::new(|_| Ok(vec![DynValue::Json(json!({}))])),
        }
        .boxed(),
    ];
    println!("out-of-order check: {:?}", bad.check().unwrap_err());

    // ── 6. Graph building derives edges from the dynamic pipeline ────────────
    // Two chained DynNodes: greeting -> shout -> report. The edge is discovered
    // purely from pointer identity, so viz and the parallel runner work too.
    let chained: StepVec<PondError> = vec![
        DynNode {
            name: "upper",
            inputs: vec![cat.get("greeting")?],
            outputs: vec![cat.get("shout")?],
            func: Box::new(|args| Ok(vec![DynValue::Text(args[0].as_text()?.to_uppercase())])),
        }
        .boxed(),
        DynNode {
            name: "wrap",
            inputs: vec![cat.get("shout")?],
            outputs: vec![cat.get("report")?],
            func: Box::new(|args| Ok(vec![DynValue::Json(json!({ "v": args[0].as_text()? }))])),
        }
        .boxed(),
    ];
    let graph = pondrs::build_pipeline_graph(&chained, &cat, &());
    println!(
        "graph: {} node(s), {} edge(s), {} source dataset(s)",
        graph.nodes.len(),
        graph.edges.len(),
        graph.source_datasets.len(),
    );
    for e in &graph.edges {
        println!(
            "  edge {} -> {} via {:?}",
            graph.nodes[e.from_node].name,
            graph.nodes[e.to_node].name,
            graph.dataset_names.get(&e.dataset.id),
        );
    }

    // ── 7. Type mismatches are caught at save time with a real message ───────
    let mismatch = cat.get("report")?.save(DynValue::Text("not json".into()));
    println!("save mismatch: {}", mismatch.unwrap_err());

    Ok(())
}
