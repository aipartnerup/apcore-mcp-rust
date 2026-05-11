# Task: add-registry-listener-tests

## Goal

Write comprehensive unit tests for `RegistryListener`, covering tool registration, unregistration, thread safety, and idempotent start/stop.

## Files Involved

- `src/server/listener.rs` — add `#[cfg(test)] mod tests` section

## Steps (TDD-first)

1. **Create mock/stub registry and factory** for testing:
   - Mock registry that supports `on(event, callback)` and can fire events manually.
   - Mock factory with a `build_tool()` that returns a known `Value`.
2. **Write test: `test_tools_empty_on_new`**
   - Create a listener with mock registry and factory.
   - Assert `tools()` returns an empty HashMap.
3. **Write test: `test_register_adds_tool`**
   - Start the listener.
   - Fire a "register" event with module_id "my-tool".
   - Mock `registry.get_definition("my-tool")` to return a descriptor.
   - Assert `tools()` contains "my-tool".
4. **Write test: `test_unregister_removes_tool`**
   - Start the listener, fire register event, then fire unregister event.
   - Assert `tools()` no longer contains the tool.
5. **Write test: `test_unregister_nonexistent_is_silent`**
   - Start the listener, fire unregister for a tool that was never registered.
   - Assert no panic and `tools()` is empty.
6. **Write test: `test_register_missing_definition_logs_warning`**
   - Mock `get_definition()` to return `None`.
   - Fire register event.
   - Assert `tools()` is empty (tool not added).
7. **Write test: `test_start_is_idempotent`**
   - Call `start()` twice.
   - Assert no panic and callbacks are not duplicated.
8. **Write test: `test_stop_causes_events_to_be_ignored`**
   - Start, then stop the listener.
   - Fire a register event.
   - Assert `tools()` is empty (event was ignored).
9. **Write test: `test_tools_returns_snapshot`**
   - Register a tool, get snapshot, register another tool.
   - Assert first snapshot does not contain the second tool (it is a clone, not a live reference).
10. **Write test: `test_concurrent_register_unregister`** (optional, stress test)
    - Spawn multiple tasks that register and unregister concurrently.
    - Assert no panics and final state is consistent.
11. **Run `cargo test`.**

## Acceptance Criteria

- [ ] At least 8 unit tests covering the scenarios above
- [ ] Tests use mock/stub registry and factory (no real apcore dependency needed)
- [ ] Tests verify thread-safety by checking snapshot isolation
- [ ] Tests verify idempotent start/stop
- [ ] All tests pass

## Dependencies

- implement-registry-listener

## Estimated Time

1 hour
