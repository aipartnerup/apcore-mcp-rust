# Task: integration

## Goal

Write cross-cutting integration tests that verify all constants work together, ensure wire-format compatibility with the Python implementation, and confirm the module compiles cleanly as a whole.

## Files Involved

- `src/constants.rs` -- add integration-level tests to the `tests` module
- `tests/constants_integration.rs` (optional) -- top-level integration test if preferred

## Steps

1. **Write integration tests** in `src/constants.rs` `tests` module:
   ```rust
   /// Verify every Python ERROR_CODES key can be parsed into an ErrorCode variant.
   #[test]
   fn all_python_error_codes_parse() {
       let python_codes = [
           "MODULE_NOT_FOUND",
           "MODULE_DISABLED",
           "SCHEMA_VALIDATION_ERROR",
           "ACL_DENIED",
           "CALL_DEPTH_EXCEEDED",
           "CIRCULAR_CALL",
           "CALL_FREQUENCY_EXCEEDED",
           "INTERNAL_ERROR",
           "MODULE_TIMEOUT",
           "MODULE_LOAD_ERROR",
           "MODULE_EXECUTE_ERROR",
           "GENERAL_INVALID_INPUT",
           "APPROVAL_DENIED",
           "APPROVAL_TIMEOUT",
           "APPROVAL_PENDING",
           "VERSION_INCOMPATIBLE",
           "ERROR_CODE_COLLISION",
           "EXECUTION_CANCELLED",
       ];
       for code_str in &python_codes {
           let parsed: ErrorCode = code_str.parse()
               .unwrap_or_else(|_| panic!("Failed to parse Python error code: {code_str}"));
           assert_eq!(&parsed.to_string(), *code_str);
       }
       assert_eq!(python_codes.len(), ErrorCode::iter().count());
   }

   /// Verify RegistryEvent wire values match Python REGISTRY_EVENTS dict values.
   #[test]
   fn registry_events_match_python() {
       assert_eq!(RegistryEvent::Register.to_string(), "register");
       assert_eq!(RegistryEvent::Unregister.to_string(), "unregister");
       assert_eq!(RegistryEvent::Register.key(), "REGISTER");
       assert_eq!(RegistryEvent::Unregister.key(), "UNREGISTER");
   }

   /// Verify MODULE_ID_PATTERN matches the Python regex exactly.
   #[test]
   fn module_id_pattern_matches_python() {
       assert_eq!(MODULE_ID_PATTERN, r"^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$");
   }

   /// ErrorCode JSON output is a plain string, not an object.
   #[test]
   fn error_code_json_is_plain_string() {
       let json = serde_json::to_value(ErrorCode::InternalError).unwrap();
       assert!(json.is_string());
       assert_eq!(json.as_str().unwrap(), "INTERNAL_ERROR");
   }
   ```

2. **Run the full test suite**:
   ```bash
   cargo test
   ```

3. **Run clippy with strict warnings**:
   ```bash
   cargo clippy -- -D warnings
   ```

4. **Run doc tests** (if any doc examples were added):
   ```bash
   cargo test --doc
   ```

5. **Verify no unused imports or dead code warnings**:
   ```bash
   cargo check 2>&1 | grep -i warning || echo "No warnings"
   ```

6. **Remove `#![allow(unused)]`** from the top of `constants.rs` if it was carried over from the stub.

## Acceptance Criteria

- [ ] All 18 Python error code strings parse into `ErrorCode` variants and round-trip
- [ ] `RegistryEvent` wire values match Python `REGISTRY_EVENTS` dict
- [ ] `MODULE_ID_PATTERN` string is identical to the Python regex pattern
- [ ] `ErrorCode` serializes to a plain JSON string (not a JSON object)
- [ ] `cargo test` passes with zero failures
- [ ] `cargo clippy -- -D warnings` passes with zero warnings
- [ ] No `#![allow(unused)]` left in the file
- [ ] `cargo doc --no-deps` builds without warnings

## Dependencies

- **Depends on:** error-codes, registry-events, patterns
- **Required by:** nothing (final task)

## Estimated Time

15 minutes
