# Task: output-formatting

## Goal

Implement the output formatting logic that converts execution results into text for LLM consumption, with support for a configurable formatter and a JSON fallback default.

## Files Involved

- `src/server/router.rs` — Add `OutputFormatter` type alias and `format_result` method

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_format_result_default_json` — With no custom formatter, a `Value::Object` is serialized to compact JSON
   - `test_format_result_default_string` — A `Value::String` is serialized as a JSON string (with quotes)
   - `test_format_result_default_number` — A `Value::Number` serializes correctly
   - `test_format_result_default_null` — `Value::Null` serializes to `"null"`
   - `test_format_result_custom_formatter` — Custom formatter is called for `Value::Object`
   - `test_format_result_custom_formatter_non_object_ignored` — Custom formatter is NOT called for non-object values; falls back to JSON
   - `test_format_result_custom_formatter_panics_fallback` — If custom formatter returns an error/panics, falls back to JSON default

2. **Define `OutputFormatter` type alias**:
   ```rust
   pub type OutputFormatter = Box<dyn Fn(&Value) -> Result<String, Box<dyn std::error::Error>> + Send + Sync>;
   ```

3. **Implement `format_result` as a method on `ExecutionRouter`**:
   ```rust
   fn format_result(&self, result: &Value) -> String {
       if let Some(ref formatter) = self.output_formatter {
           if result.is_object() {
               match formatter(result) {
                   Ok(text) => return text,
                   Err(e) => {
                       tracing::debug!("output_formatter failed, falling back to json: {e}");
                   }
               }
           }
       }
       serde_json::to_string(result).unwrap_or_else(|_| "null".to_string())
   }
   ```

4. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Default formatting uses `serde_json::to_string`
- [ ] Custom formatter is invoked only for `Value::Object` results
- [ ] Custom formatter failure falls back to JSON with debug-level log
- [ ] Non-object values always use JSON serialization
- [ ] `OutputFormatter` type alias is defined and exported
- [ ] All tests pass, clippy clean

## Dependencies

- none

## Estimated Time

45 minutes
