# Task: Unit Tests

## Goal

Write comprehensive unit tests covering all types, serialization, and both helper functions under all code paths (callback present, absent, wrong type).

## Files Involved

- `src/helpers.rs` — `#[cfg(test)] mod tests` block at the bottom of the file

## Steps (TDD-first)

Note: Several tests will already have been written during previous tasks. This task consolidates, reviews, and fills any gaps.

1. **Review existing tests** from previous tasks and ensure they are all present in the test module.
2. **Type serialization tests**:
   - `ElicitAction::Accept` serializes to `"accept"`, and deserializes from `"accept"`
   - `ElicitAction::Decline` serializes to `"decline"`
   - `ElicitAction::Cancel` serializes to `"cancel"`
   - `ElicitResult` with content serializes/deserializes correctly
   - `ElicitResult` without content (`content: None`) serializes correctly
   - Deserialization of unknown action string fails gracefully
3. **Constant value tests**:
   - `MCP_PROGRESS_KEY == "_mcp_progress"`
   - `MCP_ELICIT_KEY == "_mcp_elicit"`
4. **`report_progress` tests**:
   - With callback: verify callback receives correct `(progress, total, message)` arguments (use `Arc<Mutex<Vec<...>>>` to capture calls)
   - Without callback (empty data): no panic, returns `()`
   - With wrong type at key: no panic, returns `()`
5. **`elicit` tests**:
   - With callback returning `Accept` + content: returns `Some(ElicitResult { action: Accept, content: Some(...) })`
   - With callback returning `Decline` + no content: returns `Some(ElicitResult { action: Decline, content: None })`
   - With callback returning `Cancel`: returns `Some(ElicitResult { action: Cancel, ... })`
   - With callback returning `None`: returns `None`
   - Without callback (empty data): returns `None`
   - With wrong type at key: returns `None`
6. **Run `cargo test`** — all tests pass.
7. **Run `cargo clippy`** — no warnings.

## Acceptance Criteria

- [ ] All `ElicitAction` variants have serde round-trip tests
- [ ] `ElicitResult` has serde round-trip tests (with and without content)
- [ ] Constants have value assertion tests
- [ ] `report_progress` has tests for callback-present, callback-absent, and wrong-type paths
- [ ] `elicit` has tests for all three actions, None return, callback-absent, and wrong-type paths
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes

## Dependencies

- implement-report-progress
- implement-elicit

## Estimated Time

60 minutes
