# Task: error-mapper

## Goal

Implement `ErrorMapper` that converts apcore `ModuleError` to MCP error response dicts, sanitizing internal and ACL error codes, formatting validation errors, handling approval-related codes, and attaching AI guidance fields in camelCase.

## Files Involved

- `src/adapters/errors.rs` — Full implementation replacing the current stub
- `tests/adapters/errors_test.rs` or inline `#[cfg(test)]` module

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_internal_error_sanitized` — `CallDepthExceeded` code produces generic "Internal error occurred" with no details
   - `test_circular_call_sanitized` — `CircularCall` code produces generic message
   - `test_call_frequency_sanitized` — `CallFrequencyExceeded` code produces generic message
   - `test_acl_denied_sanitized` — `AclDenied` produces "Access denied" with no details
   - `test_schema_validation_formatted` — `SchemaValidationError` with field errors produces formatted multi-line message
   - `test_schema_validation_empty_errors` — Falls back to "Schema validation failed"
   - `test_approval_pending_narrowed` — `ApprovalPending` only passes `approvalId` (camelCase) in details
   - `test_approval_timeout_retryable` — `ApprovalTimeout` includes `retryable: true`
   - `test_approval_denied_reason` — `ApprovalDenied` passes through reason
   - `test_ai_guidance_fields` — Error with `ai_guidance`, `retryable`, `user_fixable`, `suggestion` all appear as camelCase in output
   - `test_ai_guidance_none_omitted` — `None` AI guidance fields are not included
   - `test_execution_cancelled` — `ExecutionCancelled` code produces specific message with `retryable: true`
   - `test_unknown_error_passthrough` — Non-internal, non-ACL codes pass through message and details
   - `test_output_keys_camel_case` — Verify output uses `isError`, `errorType`, `message`, `details`

2. **Define internal error code sets**:
   ```rust
   const INTERNAL_ERROR_CODES: &[ErrorCode] = &[
       ErrorCode::CallDepthExceeded,
       ErrorCode::CircularCall,
       ErrorCode::CallFrequencyExceeded,
   ];
   const SANITIZED_ERROR_CODES: &[ErrorCode] = &[
       ErrorCode::AclDenied,
   ];
   ```

3. **Implement `ErrorMapper::to_mcp_error`**:
   - Accept `&ModuleError` (not a generic `Exception` — Rust has typed errors)
   - Match on `error.code`:
     - Internal codes -> `{"isError": true, "errorType": code_str, "message": "Internal error occurred", "details": null}`
     - ACL codes -> `{"isError": true, "errorType": code_str, "message": "Access denied", "details": null}`
     - `SchemaValidationError` -> Format field errors from `details["errors"]`
     - `ApprovalPending` -> Narrow details to only `approvalId`
     - `ApprovalTimeout` -> Pass through with `retryable: true`
     - `ApprovalDenied` -> Pass through with `reason` extraction
     - `ExecutionCancelled` -> Specific cancelled message with `retryable: true`
     - Default -> Pass through all fields
   - After building the base result, call `attach_ai_guidance`

4. **Implement `attach_ai_guidance`**:
   - Read `error.retryable`, `error.ai_guidance`, `error.user_fixable`, `error.suggestion`
   - Write to result as `retryable`, `aiGuidance`, `userFixable`, `suggestion` (camelCase keys)
   - Skip `None` values; do not overwrite existing keys

5. **Implement `format_validation_errors`**:
   - Extract `errors` array from details
   - Format each as `"field: message"`
   - Return `"Schema validation failed:\n  field1: msg1\n  field2: msg2"`

6. **Serialize `ErrorCode` to string** for `errorType` field:
   - Use `serde_json::to_value(&error.code)` which gives `"SCREAMING_SNAKE_CASE"` string
   - Or implement a helper: `fn error_code_to_string(code: &ErrorCode) -> String`

7. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Internal error codes produce sanitized generic messages
- [ ] ACL_DENIED produces "Access denied" with null details
- [ ] SchemaValidationError formats field-level errors
- [ ] ApprovalPending narrows details to only `approvalId` (camelCase)
- [ ] ApprovalTimeout includes `retryable: true`
- [ ] ApprovalDenied passes through reason
- [ ] ExecutionCancelled produces specific message
- [ ] AI guidance fields appear in camelCase when present, omitted when None
- [ ] Output uses camelCase keys: `isError`, `errorType`, `message`, `details`
- [ ] All tests pass, clippy clean

## Dependencies

- adapter-setup

## Estimated Time

3 hours
