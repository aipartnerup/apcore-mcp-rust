# Task: add-server-unit-tests

## Goal

Write unit tests for `MCPServer` covering construction, address formatting, and lifecycle (start/wait/stop) using mock transport and factory.

## Files Involved

- `src/server/server.rs` — add `#[cfg(test)] mod tests` section

## Steps (TDD-first)

1. **Create mock transport manager** that immediately completes (no real network binding):
   - Returns `Ok(())` from `run_stdio()`, `run_streamable_http()`, `run_sse()`.
   - Respects a shutdown signal.
2. **Write test: `test_address_stdio`**
   - Create MCPServer with `TransportKind::Stdio`.
   - Assert `address() == "stdio"`.
3. **Write test: `test_address_streamable_http`**
   - Create MCPServer with `TransportKind::StreamableHttp`, host="0.0.0.0", port=9090.
   - Assert `address() == "http://0.0.0.0:9090"`.
4. **Write test: `test_address_sse`**
   - Create MCPServer with `TransportKind::Sse`, default host/port.
   - Assert `address() == "http://127.0.0.1:8000"`.
5. **Write test: `test_start_is_idempotent`**
   - Call `start()` twice with mock transport.
   - Assert no panic, only one background task spawned.
6. **Write test: `test_stop_unstarted_is_noop`**
   - Create server, call `stop()` without `start()`.
   - Assert no panic.
7. **Write test: `test_start_stop_wait_lifecycle`**
   - Start server with mock transport, stop it, then wait.
   - Assert `wait()` completes without error.
8. **Write test: `test_default_config_values`**
   - Create `MCPServerConfig::default()`.
   - Assert host="127.0.0.1", port=8000, name="apcore-mcp", transport=Stdio.
9. **Write test: `test_transport_kind_from_str`**
   - Test all valid and invalid transport strings.
10. **Run `cargo test`.**

## Acceptance Criteria

- [ ] At least 7 unit tests covering the scenarios above
- [ ] Tests use mock/stub transport (no real network binding)
- [ ] Tests verify address formatting for all transport variants
- [ ] Tests verify idempotent start and safe stop-before-start
- [ ] Tests verify full lifecycle (start -> stop -> wait)
- [ ] All tests pass

## Dependencies

- implement-server-lifecycle

## Estimated Time

1.5 hours
