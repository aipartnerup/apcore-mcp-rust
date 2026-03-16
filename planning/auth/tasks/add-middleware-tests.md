# Task: add-middleware-tests

## Goal

Write unit tests for `AuthMiddlewareService` covering exempt paths, 401 responses, task-local propagation, and permissive mode.

## Files Involved

- `src/auth/middleware.rs` — add `#[cfg(test)] mod tests` block

## Steps (TDD-first)

1. **Create a mock `Authenticator`** that returns a fixed `Identity` when a specific header is present, `None` otherwise.
2. **Create a simple inner service** (using `tower::service_fn`) that reads `AUTH_IDENTITY` task-local and returns it as JSON in the response body.
3. **Write tests:**
   - `test_valid_auth_forwards_request` — valid auth header, non-exempt path; verify 200 and identity in task-local.
   - `test_missing_auth_returns_401` — no auth header, `require_auth=true`; verify 401 status, JSON body, `WWW-Authenticate` header.
   - `test_exempt_path_bypasses_auth` — request to `/health` with no auth; verify request is forwarded (not 401).
   - `test_exempt_prefix_bypasses_auth` — request to `/metrics/cpu` with prefix `/metrics`; verify forwarded.
   - `test_exempt_path_with_valid_token` — exempt path with valid token; verify identity is populated (best-effort).
   - `test_exempt_path_with_invalid_token` — exempt path with bad token; verify request still forwarded (identity is None).
   - `test_permissive_mode_allows_unauthenticated` — `require_auth=false`, no token; verify request forwarded with `None` identity.
   - `test_custom_exempt_paths` — override default exempt paths.
4. **Use `tokio::test` runtime** for all async tests.
5. **Use `tower::ServiceExt::oneshot()`** for clean request dispatch.

## Acceptance Criteria

- [ ] All listed tests are implemented and pass
- [ ] Mock authenticator is simple and reusable
- [ ] Tests verify both HTTP status codes and response bodies
- [ ] Task-local propagation is verified by reading it in the inner service
- [ ] Tests use `tower::ServiceExt` for ergonomic assertions

## Dependencies

- implement-tower-middleware

## Estimated Time

1 hour
