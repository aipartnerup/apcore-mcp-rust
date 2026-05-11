# Task: implement-tower-middleware

## Goal

Implement the tower `Service` for `AuthMiddlewareService` that authenticates requests, sets the `AUTH_IDENTITY` task-local, and returns 401 on failure.

## Files Involved

- `src/auth/middleware.rs` — implement `Service` for `AuthMiddlewareService`

## Steps (TDD-first)

1. **Write a basic test** that constructs an `AuthMiddlewareLayer`, wraps a simple echo service, and sends a request with no auth header — expect 401.
2. **Add configuration fields to `AuthMiddlewareLayer`:**
   ```rust
   pub struct AuthMiddlewareLayer {
       authenticator: Arc<dyn Authenticator>,
       exempt_paths: HashSet<String>,
       exempt_prefixes: Vec<String>,
       require_auth: bool,
   }
   ```
3. **Update `AuthMiddlewareLayer::new()`** to accept exempt_paths, exempt_prefixes, require_auth with defaults:
   - `exempt_paths`: `{"/health", "/metrics"}`
   - `exempt_prefixes`: empty
   - `require_auth`: `true`
4. **Propagate fields through `Layer::layer()`** to `AuthMiddlewareService`.
5. **Add `is_exempt()` helper** on `AuthMiddlewareService`.
6. **Implement `Service<Request<B>>` for `AuthMiddlewareService<S>`:**
   - `poll_ready`: delegate to inner service.
   - `call`:
     a. Extract path from request URI.
     b. If exempt: best-effort auth (try authenticate, ignore errors), set task-local, forward.
     c. If not exempt: authenticate. If `None` and `require_auth` -> return 401 JSON response.
     d. Otherwise: set `AUTH_IDENTITY` task-local via `.scope()`, forward to inner.
7. **Implement `build_401_response()`** helper:
   - Status 401
   - `Content-Type: application/json`
   - `WWW-Authenticate: Bearer`
   - Body: `{"error": "Unauthorized", "detail": "Missing or invalid Bearer token"}`
8. **Ensure `extract_headers()` lowercases header names** (already does via `name.to_string()` on `HeaderName` which is lowercase).
9. **Remove `#![allow(unused)]`.**
10. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `AuthMiddlewareLayer` implements `tower::Layer`
- [ ] `AuthMiddlewareService` implements `tower::Service<Request<B>>`
- [ ] Exempt paths bypass auth (with best-effort identity extraction)
- [ ] Exempt prefixes work via `starts_with`
- [ ] Non-exempt paths without valid auth get 401 JSON response
- [ ] 401 response includes `WWW-Authenticate: Bearer` header
- [ ] `AUTH_IDENTITY` task-local is set for the inner service's future
- [ ] `require_auth=false` allows unauthenticated requests through

## Dependencies

- align-identity-type
- implement-jwt-authenticator

## Estimated Time

1.5 hours
