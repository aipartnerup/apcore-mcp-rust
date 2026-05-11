# Task: add-integration-tests

## Goal

Write integration tests that exercise MCPServer and RegistryListener together, verifying that dynamic tool registration flows through to the server and that the full start/stop lifecycle works end-to-end.

## Files Involved

- `tests/server_integration.rs` — new integration test file (or `src/server/server.rs` inline tests)

## Steps (TDD-first)

1. **Write test: `test_server_with_listener_register_tool`**
   - Create a mock registry and factory.
   - Create RegistryListener with the mock registry and factory.
   - Start the listener.
   - Trigger a register event on the mock registry.
   - Assert `listener.tools()` contains the registered tool.
2. **Write test: `test_server_with_listener_unregister_tool`**
   - Same setup as above.
   - Register a tool, then unregister it.
   - Assert `listener.tools()` is empty after unregister.
3. **Write test: `test_server_start_stop_with_listener`**
   - Create MCPServer with mock transport and a RegistryListener.
   - Start the server.
   - Assert server address is correct.
   - Stop the server.
   - Wait for completion.
   - Assert no errors.
4. **Write test: `test_server_address_consistency`**
   - Create servers with each transport kind.
   - Verify address is correct before and after start().
5. **Write test: `test_listener_stop_during_server_stop`**
   - Verify that stopping the server also effectively stops the listener (events are ignored after server stop).
6. **Run `cargo test`.**

## Acceptance Criteria

- [ ] At least 4 integration tests
- [ ] Tests exercise MCPServer and RegistryListener together
- [ ] Tests verify dynamic tool registration through the listener
- [ ] Tests verify server lifecycle does not leave dangling tasks
- [ ] All tests pass

## Dependencies

- add-registry-listener-tests
- add-server-unit-tests

## Estimated Time

1 hour
