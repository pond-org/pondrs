# Datasets

```rust,no_compile
pub trait Dataset: serde::Serialize {
    type LoadItem;
    type SaveItem;
    type Error;

    fn load(&self) -> Result<Self::LoadItem, Self::Error>;
    fn save(&self, output: Self::SaveItem) -> Result<(), Self::Error>;
}
```

```rust,no_compile
#[derive(Serialize, Deserialize, Clone)]
pub struct TextDataset {
    path: String,
}

impl Dataset for TextDataset {
    type LoadItem = String;
    type SaveItem = String;
    type Error = PondError;

    fn save(&self, text: Self::SaveItem) -> Result<(), PondError> {
        std::fs::write(&self.path, text)?;
        Ok(())
    }

    fn load(&self) -> Result<Self::LoadItem, PondError> {
        Ok(std::fs::read_to_string(&self.path)?)
    }
}
```
