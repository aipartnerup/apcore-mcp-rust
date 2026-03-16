# Task: integration-wiring

## Objective
Update `lib.rs` re-exports, verify the full module compiles, and ensure public API surface matches the feature spec.

## Estimate
~30 min

## TDD Tests (write first)

```rust
// tests/apcore_mcp_api.rs (integration test)

#[test]
fn public_api_exports_exist() {
    // Verify all expected types are accessible from crate root
    let _ = std::any::type_name::<apcore_mcp::APCoreMCP>();
    let _ = std::any::type_name::<apcore_mcp::APCoreMCPBuilder>();
    let _ = std::any::type_name::<apcore_mcp::BackendSource>();
    let _ = std::any::type_name::<apcore_mcp::APCoreMCPError>();

    // Verify convenience functions are importable
    use apcore_mcp::{serve, async_serve, to_openai_tools};
}

#[test]
fn builder_is_accessible_from_struct() {
    let _builder = apcore_mcp::APCoreMCP::builder();
}
```

## Implementation Steps
1. Add re-exports to `lib.rs`:
   - `pub use crate::apcore_mcp::BackendSource;`
   - `pub use crate::apcore_mcp::APCoreMCPError;`
   - `pub use crate::apcore_mcp::APCoreMCPConfig;`
   - Any new config structs (ServeOptions, etc.)
2. Run `cargo check` to verify compilation
3. Run `cargo test` to verify all tests pass
4. Run `cargo doc` to verify documentation renders
5. Audit public API surface against feature spec acceptance criteria
6. Add any missing doc comments

## Acceptance Criteria
- [ ] `cargo check` passes with no errors
- [ ] `cargo test` passes all new tests
- [ ] All types listed in feature spec are publicly accessible
- [ ] Doc comments present on all public items
- [ ] No unnecessary `#![allow(unused)]` remaining on implemented items

## Dependencies
- All other tasks complete

## Files Modified
- `src/lib.rs`
- `src/apcore_mcp.rs` (doc comments cleanup)
