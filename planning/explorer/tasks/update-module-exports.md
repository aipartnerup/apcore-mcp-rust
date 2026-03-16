# Task: update-module-exports

## Goal

Clean up `src/explorer/mod.rs` to declare all submodules and re-export the public API surface. Remove any remaining `#![allow(unused)]` directives. Verify the module integrates cleanly with the rest of the crate.

## Files Involved

- `src/explorer/mod.rs` — update module declarations and re-exports
- `src/lib.rs` — verify `pub mod explorer;` is present

## Steps (TDD-first)

1. **Write a compile-time test** (doc test or unit test) that imports `apcore_mcp::explorer::create_explorer_mount` and `apcore_mcp::explorer::ExplorerConfig` to verify public API accessibility.

2. **Update `src/explorer/mod.rs`:**
   ```rust
   //! Explorer sub-module — introspection mount for browsing registered tools.

   pub mod api;
   pub mod mount;
   pub mod templates;

   pub use mount::{create_explorer_mount, ExplorerConfig, ToolInfo, HandleCallFn, CallResult};
   pub use api::{ExplorerState, CallResponse};
   ```

3. **Verify `src/lib.rs`** has `pub mod explorer;`.

4. **Remove all `#![allow(unused)]`** from explorer files.

5. **Run `cargo check` and `cargo test`.**

6. **Run `cargo doc --no-deps`** to verify documentation renders.

## Acceptance Criteria

- [ ] `mod.rs` declares all submodules: `api`, `mount`, `templates`
- [ ] Public types are re-exported at the `explorer` module level
- [ ] `use apcore_mcp::explorer::create_explorer_mount` compiles from external code
- [ ] No `#![allow(unused)]` remains in explorer files
- [ ] No `todo!()` remains in explorer files
- [ ] `cargo check` passes with no warnings
- [ ] `cargo test` passes

## Dependencies

- add-explorer-tests

## Estimated Time

20 minutes
