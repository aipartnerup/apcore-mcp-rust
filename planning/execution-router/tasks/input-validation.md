# Task: input-validation

## Goal

Implement pre-execution input validation that calls `executor.validate()` before dispatching to the execution path, returning a validation error response when inputs are invalid. This is only active when `validate_inputs` is `true` on the router.

## Files Involved

- `src/server/router.rs` — Add validation logic to `handle_call` flow

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_validation_disabled_skips` — When `validate_inputs = false`, executor.validate() is never called even if inputs are invalid
   - `test_validation_enabled_valid_inputs` — When validate returns `valid = true`, execution proceeds normally
   - `test_validation_enabled_invalid_inputs` — When validate returns `valid = false` with errors, returns error content without calling executor
   - `test_validation_error_format_single_field` — Single field error: `"Validation failed: field_name: message"`
   - `test_validation_error_format_multiple_fields` — Multiple field errors joined by `"; "`
   - `test_validation_error_format_nested_errors` — Nested errors (error with sub-errors) are flattened: each sub-error's `field: message`
   - `test_validation_error_format_no_field` — Error without field uses message or code fallback
   - `test_validation_executor_lacks_validate` — When executor.validate() returns None, validation is skipped
   - `test_validation_exception_mapped` — If validate() itself throws/returns error, it's mapped via ErrorMapper
   - `test_validation_returns_is_error_true` — Validation failure returns `is_error = true` and `trace_id = None`

2. **Implement validation error formatting**:
   ```rust
   fn format_validation_errors(errors: &[ValidationError]) -> String {
       let parts: Vec<String> = errors
           .iter()
           .flat_map(|e| {
               if !e.errors.is_empty() {
                   e.errors.iter().map(|sub| {
                       format!("{}: {}",
                           sub.field.as_deref().unwrap_or("?"),
                           &sub.message)
                   }).collect::<Vec<_>>()
               } else if let Some(ref field) = e.field {
                   vec![format!("{field}: {}", &e.message)]
               } else {
                   vec![e.message.clone()]
               }
           })
           .collect();
       parts.join("; ")
   }
   ```

3. **Add validation step to `handle_call`**:
   - Check `self.validate_inputs`
   - Call `self.executor.validate(tool_name, arguments, context)`
   - If returns `Some(result)` and `!result.valid`:
     - Format errors via `format_validation_errors`
     - Return `([text: "Validation failed: {detail}"], true, None)`
   - If returns `None`, skip validation
   - If validate panics/errors, map via ErrorMapper

4. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Validation is skipped when `validate_inputs = false`
- [ ] Validation is skipped when executor.validate() returns `None`
- [ ] Invalid inputs produce an error response without calling executor
- [ ] Validation error messages format field errors as `"field: message"` joined by `"; "`
- [ ] Nested errors are flattened (sub-errors extracted)
- [ ] Errors without field names fall back to message text
- [ ] Validation exceptions are mapped via ErrorMapper
- [ ] Validation failure returns `is_error = true` and `trace_id = None`
- [ ] All tests pass, clippy clean

## Dependencies

- non-streaming-path (for ErrorMapper integration and content structure)

## Estimated Time

1 hour
