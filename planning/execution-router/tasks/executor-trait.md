# Task: executor-trait

## Goal

Define the `Executor` trait that abstracts the apcore execution pipeline, providing async `call_async`, optional `stream`, and optional `validate` methods. This trait replaces Python's duck-typing approach with compile-time contracts and must be object-safe for use as `dyn Executor`.

## Files Involved

- `src/server/router.rs` — Add `Executor` trait, `ExecutorError`, `ValidationResult` types
- `Cargo.toml` — Add `tokio-stream` dependency if not present

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_mock_executor_call_async` — A mock implementing `Executor` can be called and returns a `Value`
   - `test_mock_executor_stream_none` — A mock can return `None` for `stream()` indicating no streaming support
   - `test_mock_executor_stream_some` — A mock can return a `Stream` of `Value` chunks
   - `test_mock_executor_validate_none` — A mock can return `None` for `validate()` indicating no validation support
   - `test_mock_executor_validate_result` — A mock can return a `ValidationResult` with valid/invalid state
   - `test_executor_is_object_safe` — Verify `Box<dyn Executor>` compiles (compile-time test)

2. **Add `tokio-stream` dependency**:
   ```toml
   tokio-stream = "0.1"
   ```

3. **Define `ExecutorError`**:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum ExecutorError {
       #[error("{message}")]
       Execution { code: String, message: String, details: Option<Value> },
       #[error("validation failed: {0}")]
       Validation(String),
       #[error("{0}")]
       Other(#[from] Box<dyn std::error::Error + Send + Sync>),
   }
   ```

4. **Define `ValidationResult`**:
   ```rust
   #[derive(Debug, Clone)]
   pub struct ValidationResult {
       pub valid: bool,
       pub errors: Vec<ValidationError>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ValidationError {
       pub field: Option<String>,
       pub message: String,
       #[serde(default, skip_serializing_if = "Vec::is_empty")]
       pub errors: Vec<ValidationError>,
   }
   ```

5. **Define `Executor` trait**:
   ```rust
   #[async_trait]
   pub trait Executor: Send + Sync {
       async fn call_async(
           &self,
           module_id: &str,
           inputs: &Value,
           context: Option<&Context>,
       ) -> Result<Value, ExecutorError>;

       fn stream(
           &self,
           module_id: &str,
           inputs: &Value,
           context: Option<&Context>,
       ) -> Option<Pin<Box<dyn Stream<Item = Result<Value, ExecutorError>> + Send>>>;

       fn validate(
           &self,
           module_id: &str,
           inputs: &Value,
           context: Option<&Context>,
       ) -> Option<ValidationResult>;
   }
   ```
   - `stream` and `validate` have default implementations returning `None`

6. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] `Executor` trait is defined with `call_async`, `stream`, `validate`
- [ ] `stream` and `validate` have default `None` implementations
- [ ] `ExecutorError` enum covers execution, validation, and generic errors
- [ ] `ValidationResult` and `ValidationError` types are defined
- [ ] Trait is object-safe (`Box<dyn Executor>` compiles)
- [ ] All methods accept `Option<&Context>` (no runtime introspection needed)
- [ ] `tokio-stream` added to `Cargo.toml`
- [ ] All tests pass, clippy clean

## Dependencies

- none

## Estimated Time

1.5 hours
