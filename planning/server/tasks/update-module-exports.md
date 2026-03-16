# Task: update-module-exports

## Goal

Clean up `src/server/mod.rs` to re-export the key public types, remove all `#![allow(unused)]` directives from server module files, and verify no stale `todo!()` macros remain.

## Files Involved

- `src/server/mod.rs` — add public re-exports
- `src/server/server.rs` — remove `#![allow(unused)]`
- `src/server/listener.rs` — remove `#![allow(unused)]`

## Steps (TDD-first)

1. **Write a compile test** that imports all public types from `crate::server`:
   ```rust
   use crate::server::{MCPServer, MCPServerConfig, TransportKind, RegistryOrExecutor, RegistryListener};
   ```
2. **Update `src/server/mod.rs`:**
   ```rust
   pub mod factory;
   pub mod router;
   pub mod transport;
   pub mod listener;
   pub mod server;

   pub use self::server::{MCPServer, MCPServerConfig, TransportKind, RegistryOrExecutor};
   pub use self::listener::RegistryListener;
   ```
3. **Remove `#![allow(unused)]`** from `server.rs` and `listener.rs`.
4. **Search for remaining `todo!()` macros** in `server.rs` and `listener.rs`. If any remain, they indicate incomplete work — flag them.
5. **Run `cargo check --all-targets`** to verify no warnings.
6. **Run `cargo test`** to verify all tests still pass.

## Acceptance Criteria

- [ ] All key types are re-exported from `crate::server`
- [ ] No `#![allow(unused)]` in `server.rs` or `listener.rs`
- [ ] No `todo!()` macros in `server.rs` or `listener.rs`
- [ ] `cargo check` produces no warnings for the server module
- [ ] All tests pass

## Dependencies

- add-integration-tests

## Estimated Time

20 minutes
