# Task: add-explorer-tests

## Goal

Add comprehensive unit and integration tests for the explorer module. While each prior task includes inline TDD tests, this task adds end-to-end integration tests that exercise the full stack: HTML serving, tool listing, tool execution with and without auth, and error cases.

## Files Involved

- `src/explorer/mount.rs` — integration tests at bottom of file (or `tests/explorer.rs` if preferred)
- `src/explorer/api.rs` — additional edge-case unit tests

## Steps (TDD-first)

1. **Integration test: full lifecycle.**
   - Create `ExplorerConfig` with two tools, a mock `handle_call`, and `allow_execute=true`.
   - Build the router with `create_explorer_mount`.
   - Use `tower::ServiceExt::oneshot` to send requests.
   - `GET /` -> 200, content-type text/html, body contains tool explorer title.
   - `GET /tools` -> 200, JSON array with 2 entries.
   - `POST /tools/tool_one/call` with `{"arg": "value"}` -> 200, mock result.
   - `POST /tools/nonexistent/call` -> 404.

2. **Integration test: execution disabled.**
   - Config with `allow_execute=false`.
   - `POST /tools/tool_one/call` -> 403 with JSON error body.
   - `GET /tools` -> 200 (listing still works).

3. **Integration test: auth required.**
   - Config with `allow_execute=true` and a mock `Authenticator`.
   - `POST /tools/tool_one/call` without Authorization header -> 401.
   - `POST /tools/tool_one/call` with valid Bearer token -> 200.
   - Verify `AUTH_IDENTITY` was set during execution (mock handle_call checks task-local).

4. **Unit test: `ToolInfo` serialization.**
   - Verify `inputSchema` field name in JSON output (camelCase).

5. **Unit test: `render_html` edge cases.**
   - Empty project_name and project_url -> no footer link.
   - Special characters in title are escaped.

6. **Run full test suite: `cargo test`.**

## Acceptance Criteria

- [ ] Integration tests cover: HTML serving, tool listing, tool execution, 403, 404, 401
- [ ] Auth integration test verifies AUTH_IDENTITY is set during tool execution
- [ ] All tests pass with `cargo test`
- [ ] No test relies on network or external services
- [ ] Tests use `tower::ServiceExt::oneshot` for request dispatch

## Dependencies

- implement-explorer-mount

## Estimated Time

1.5 hours
