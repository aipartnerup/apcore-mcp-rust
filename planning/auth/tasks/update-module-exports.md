# Task: update-module-exports

## Goal

Clean up `mod.rs` re-exports to present a tidy public API surface, remove all `#![allow(unused)]` directives, and verify the module compiles cleanly.

## Files Involved

- `src/auth/mod.rs` — update re-exports

## Steps

1. **Update `src/auth/mod.rs`** to re-export all public types:
   ```rust
   pub use protocol::{Authenticator, Identity};
   pub use jwt::{ClaimMapping, JWTAuthenticator};
   pub use middleware::{AuthMiddlewareLayer, AuthMiddlewareService, AUTH_IDENTITY, extract_headers};
   ```
2. **Remove all `#![allow(unused)]`** from every file in `src/auth/`.
3. **Run `cargo check`** — verify zero warnings.
4. **Run `cargo test`** — verify all tests pass.
5. **Run `cargo doc --no-deps`** — verify doc generation succeeds and public items have doc comments.

## Acceptance Criteria

- [ ] `mod.rs` re-exports all public API items listed in the feature spec
- [ ] No `#![allow(unused)]` in any auth module file
- [ ] No `todo!()` macros remain
- [ ] `cargo check` produces zero warnings
- [ ] `cargo test` passes
- [ ] `cargo doc` generates docs without errors

## Dependencies

- add-integration-tests

## Estimated Time

20 minutes
