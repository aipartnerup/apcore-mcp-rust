# Task: cleanup-and-exports

## Goal

Remove all stub code, `#![allow(unused)]` directives, and `todo!()` macros from `transport.rs`. Verify `mod.rs` re-exports are correct. Ensure `cargo check` and `cargo test` pass with no warnings.

## Files Involved

- `src/server/transport.rs` — remove stubs and unused markers
- `src/server/mod.rs` — verify re-exports

## Steps

1. **Remove `#![allow(unused)]` from top of `transport.rs`.**
2. **Search for any remaining `todo!()` macros** — replace with real implementations or `unimplemented!()` with a tracking comment if genuinely deferred.
3. **Verify `mod.rs` exports:**
   - `pub mod transport;` should already exist.
   - Confirm `TransportManager`, `MetricsExporter`, `TransportError`, and `McpHandler` are accessible from `crate::server::transport::*`.
4. **Run `cargo check` — zero warnings.**
5. **Run `cargo test` — all tests pass.**
6. **Run `cargo clippy` — no new lints.**

## Acceptance Criteria

- [ ] No `#![allow(unused)]` in `transport.rs`
- [ ] No `todo!()` macros in `transport.rs`
- [ ] `cargo check` passes with no warnings
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes
- [ ] Public types are properly exported via `mod.rs`

## Dependencies

- add-transport-integration-tests

## Estimated Time

20 minutes
