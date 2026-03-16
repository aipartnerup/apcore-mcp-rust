# Task: adapter-setup

## Goal

Scaffold the shared infrastructure for all adapter modules: define `AdapterError` enum, MCP response types, update `mod.rs` re-exports, and establish the test harness.

## Files Involved

- `src/adapters/mod.rs` — Add `AdapterError` enum, re-export public types
- `src/constants.rs` — Verify/extend error code constants if needed

## Steps (TDD-first)

1. **Write tests first** for `AdapterError`:
   - Test that `AdapterError::SchemaConversion` displays correctly
   - Test that `AdapterError::InvalidModuleId` includes the ID and pattern in the message

2. **Define `AdapterError` enum** in `src/adapters/mod.rs`:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum AdapterError {
       #[error("schema conversion failed: {0}")]
       SchemaConversion(String),
       #[error("invalid module ID '{id}': must match {pattern}")]
       InvalidModuleId { id: String, pattern: &'static str },
   }
   ```

3. **Define MCP response types** (optional typed structs for annotation and error output):
   - `McpToolAnnotations` struct with `#[serde(rename_all = "camelCase")]`
   - Keep these in `mod.rs` or a new `types.rs` submodule

4. **Update `mod.rs`** to re-export all public types:
   - `pub use annotations::AnnotationMapper;`
   - `pub use errors::ErrorMapper;`
   - `pub use schema::SchemaConverter;`
   - `pub use id_normalizer::ModuleIDNormalizer;`
   - `pub use approval::ElicitationApprovalHandler;`
   - `pub use AdapterError;`

5. **Verify `cargo check`** passes with updated module structure.

## Acceptance Criteria

- [ ] `AdapterError` enum compiles and implements `std::error::Error` + `Display`
- [ ] All adapter submodules are declared in `mod.rs`
- [ ] Public re-exports are in place
- [ ] `cargo check` passes
- [ ] Test module skeleton exists

## Dependencies

None.

## Estimated Time

1 hour
