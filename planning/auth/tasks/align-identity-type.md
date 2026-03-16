# Task: align-identity-type

## Goal

Replace the local `Identity` struct in `protocol.rs` with a re-export of `apcore::Identity`, aligning the Rust auth module with the core crate's identity model (matching the Python pattern where `apcore.Identity` is the canonical type).

## Files Involved

- `src/auth/protocol.rs` — remove local `Identity`, re-export `apcore::Identity`, update `Authenticator` trait
- `src/auth/jwt.rs` — update imports
- `src/auth/middleware.rs` — update imports

## Steps (TDD-first)

1. **Write a compile-test** that asserts `crate::auth::protocol::Identity` is the same type as `apcore::Identity` (use `std::any::TypeId` in a `#[test]`).
2. **Remove the local `Identity` struct** from `protocol.rs`.
3. **Add `pub use apcore::Identity;`** to `protocol.rs`.
4. **Update the `Authenticator` trait** to return `Option<Identity>` where `Identity = apcore::Identity`.
5. **Update imports** in `jwt.rs` and `middleware.rs` to use the re-exported type.
6. **Remove `#![allow(unused)]`** from `protocol.rs`.
7. **Verify** `cargo check` passes with no errors or warnings.

## Acceptance Criteria

- [ ] No local `Identity` struct in `protocol.rs`
- [ ] `apcore::Identity` is re-exported as `crate::auth::protocol::Identity`
- [ ] `Authenticator` trait compiles with `apcore::Identity`
- [ ] All files in `src/auth/` compile without errors
- [ ] Fields match Python's `Identity`: `id`, `type` (as `identity_type`), `roles`, `attrs`

## Dependencies

None

## Estimated Time

30 minutes
