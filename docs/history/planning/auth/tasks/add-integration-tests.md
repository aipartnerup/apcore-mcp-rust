# Task: add-integration-tests

## Goal

Write end-to-end integration tests that wire `JWTAuthenticator` with `AuthMiddlewareLayer` on an axum router, verifying the full auth flow from HTTP request to task-local identity.

## Files Involved

- `tests/auth_integration.rs` (new file) — or `src/auth/mod.rs` integration test block

## Steps (TDD-first)

1. **Build an axum Router** with:
   - A protected endpoint (`GET /api/whoami`) that reads `AUTH_IDENTITY` and returns the identity as JSON.
   - The `AuthMiddlewareLayer` wrapping the router with a `JWTAuthenticator`.
2. **Write tests:**
   - `test_full_flow_valid_jwt` — create a real JWT, send request with Bearer token, verify 200 and correct identity JSON.
   - `test_full_flow_no_token` — send request without token; verify 401.
   - `test_full_flow_exempt_health` — `GET /health` returns 200 without token.
   - `test_full_flow_permissive_mode` — `require_auth=false`, no token; verify 200 with no identity.
3. **Use `axum::body::to_bytes()`** to read response bodies.
4. **Use `tower::ServiceExt::oneshot()`** on the axum router (no need for actual TCP listener).

## Acceptance Criteria

- [ ] Integration tests exercise the full stack: axum -> tower middleware -> JWT authenticator -> task-local -> handler
- [ ] Tests are self-contained (no external services or env vars required)
- [ ] Tests pass with `cargo test`

## Dependencies

- add-jwt-authenticator-tests
- add-middleware-tests

## Estimated Time

1 hour
