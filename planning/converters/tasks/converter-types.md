# Task: converter-types

## Goal

Define the `ConverterError` enum, scaffold the `OpenAIConverter` struct with adapter composition, and set up module exports in `mod.rs`.

## Files Involved

- `src/converters/openai.rs` — Replace stub with struct definition and adapter fields
- `src/converters/mod.rs` — Update re-exports

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_converter_error_display_adapter` — `ConverterError::Adapter` formats correctly
   - `test_converter_error_display_strict` — `ConverterError::StrictMode` formats correctly
   - `test_openai_converter_new` — `OpenAIConverter::new()` creates instance without panic

2. **Define `ConverterError`**:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum ConverterError {
       #[error("adapter error: {0}")]
       Adapter(#[from] crate::adapters::AdapterError),
       #[error("strict mode conversion failed: {0}")]
       StrictMode(String),
   }
   ```

3. **Define `OpenAIConverter` struct**:
   ```rust
   pub struct OpenAIConverter {
       schema_converter: SchemaConverter,
       annotation_mapper: AnnotationMapper,
       id_normalizer: ModuleIDNormalizer,
   }

   impl OpenAIConverter {
       pub fn new() -> Self {
           Self {
               schema_converter: SchemaConverter,
               annotation_mapper: AnnotationMapper,
               id_normalizer: ModuleIDNormalizer,
           }
       }
   }
   ```

4. **Update `mod.rs`** to re-export `OpenAIConverter` and `ConverterError`.

5. **Replace `todo!()` method stubs** with proper signatures returning `Result`:
   - `convert_registry(&self, registry: &Registry, ...) -> Result<Vec<Value>, ConverterError>`
   - `convert_descriptor(&self, name: &str, descriptor: &ModuleDescriptor, description: &str, ...) -> Result<Value, ConverterError>`
   - Keep method bodies as `todo!()` for now — subsequent tasks will implement them.

6. **Run tests** — ensure compilation and initial tests pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] `ConverterError` enum defined with `Adapter` and `StrictMode` variants
- [ ] `ConverterError` implements `Debug`, `Display`, `Error` (via thiserror)
- [ ] `OpenAIConverter` struct holds adapter instances
- [ ] `OpenAIConverter::new()` constructs without panic
- [ ] Method signatures defined with proper types (returning `Result`)
- [ ] `mod.rs` re-exports public types
- [ ] All tests pass, clippy clean

## Dependencies

- none (adapters module must exist as stubs, which it already does)

## Estimated Time

1 hour
