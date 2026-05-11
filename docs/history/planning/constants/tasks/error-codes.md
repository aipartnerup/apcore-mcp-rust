# Task: error-codes

## Goal

Implement the `ErrorCode` enum with all 18 variants matching the Python reference, with `Display`, `FromStr`, `Serialize`, `Deserialize`, and `EnumIter` derives.

## Files Involved

- `src/constants.rs` -- add `ErrorCode` enum definition

## Steps

1. **Write tests first** (TDD) -- add to the `tests` module in `src/constants.rs`:
   ```rust
   #[test]
   fn error_code_display_round_trip() {
       for code in ErrorCode::iter() {
           let s = code.to_string();
           let parsed: ErrorCode = s.parse().unwrap();
           assert_eq!(parsed, code);
       }
   }

   #[test]
   fn error_code_serde_round_trip() {
       for code in ErrorCode::iter() {
           let json = serde_json::to_string(&code).unwrap();
           let parsed: ErrorCode = serde_json::from_str(&json).unwrap();
           assert_eq!(parsed, code);
       }
   }

   #[test]
   fn error_code_count() {
       assert_eq!(ErrorCode::iter().count(), 18);
   }

   #[test]
   fn error_code_known_values() {
       // Spot-check a few
       assert_eq!(ErrorCode::ModuleNotFound.to_string(), "MODULE_NOT_FOUND");
       assert_eq!(ErrorCode::SchemaValidationError.to_string(), "SCHEMA_VALIDATION_ERROR");
       assert_eq!(ErrorCode::ExecutionCancelled.to_string(), "EXECUTION_CANCELLED");
   }

   #[test]
   fn error_code_from_str_invalid() {
       assert!("NOT_A_REAL_CODE".parse::<ErrorCode>().is_err());
   }
   ```

2. **Run tests -- expect compile failure** (enum not yet defined):
   ```bash
   cargo test -- constants
   ```

3. **Implement the enum** in `src/constants.rs`:
   ```rust
   /// Standard error codes emitted by the apcore MCP bridge.
   ///
   /// Each variant serializes to its SCREAMING_SNAKE_CASE string form
   /// for wire-format compatibility with other language SDKs.
   #[derive(
       Debug, Clone, Copy, PartialEq, Eq, Hash,
       Display, EnumString, EnumIter, IntoStaticStr,
       Serialize, Deserialize,
   )]
   #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
   #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
   #[non_exhaustive]
   pub enum ErrorCode {
       ModuleNotFound,
       ModuleDisabled,
       SchemaValidationError,
       AclDenied,
       CallDepthExceeded,
       CircularCall,
       CallFrequencyExceeded,
       InternalError,
       ModuleTimeout,
       ModuleLoadError,
       ModuleExecuteError,
       GeneralInvalidInput,
       ApprovalDenied,
       ApprovalTimeout,
       ApprovalPending,
       VersionIncompatible,
       ErrorCodeCollision,
       ExecutionCancelled,
   }
   ```

4. **Run tests -- expect all to pass**:
   ```bash
   cargo test -- constants
   ```

5. **Run clippy**:
   ```bash
   cargo clippy -- -D warnings
   ```

## Acceptance Criteria

- [ ] `ErrorCode` has exactly 18 variants matching all Python `ERROR_CODES` keys
- [ ] `Display` produces SCREAMING_SNAKE_CASE (e.g., `"MODULE_NOT_FOUND"`)
- [ ] `FromStr` parses SCREAMING_SNAKE_CASE back to the correct variant
- [ ] `serde_json` serializes/deserializes to/from the same string
- [ ] `EnumIter` allows iteration over all variants
- [ ] `#[non_exhaustive]` is present for forward compatibility
- [ ] Invalid strings return `Err` from `FromStr`
- [ ] All tests pass

## Dependencies

- **Depends on:** setup
- **Required by:** integration

## Estimated Time

20 minutes
