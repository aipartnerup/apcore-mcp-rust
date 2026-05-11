# Task: Integration Smoke Test

## Goal

Write an end-to-end smoke test that simulates the full lifecycle: creating a context with injected callbacks, calling `report_progress` and `elicit`, and verifying that callbacks are invoked with correct arguments and return values propagate.

## Files Involved

- `tests/helpers_integration.rs` (new integration test file) — or `src/helpers.rs` if kept as an extended test module

## Steps (TDD-first)

1. **Create integration test**: Set up a mock context with a `data` HashMap containing both a `ProgressCallback` and an `ElicitCallback`.
2. **Test progress flow**:
   - Inject a `ProgressCallback` that records calls to a shared `Arc<Mutex<Vec<(f64, Option<f64>, Option<String>)>>>`.
   - Call `report_progress(&context, 50.0, Some(100.0), Some("halfway"))`.
   - Assert the recorded call matches expected arguments.
3. **Test elicit flow**:
   - Inject an `ElicitCallback` that returns `Some(ElicitResult { action: Accept, content: Some(json!({"name": "test"})) })`.
   - Call `elicit(&context, "Enter name:", Some(&schema))`.
   - Assert the returned `ElicitResult` has `action == Accept` and expected content.
4. **Test no-op flow**:
   - Create a context with no callbacks in data.
   - Call both `report_progress` and `elicit`.
   - Assert no panic; `elicit` returns `None`.
5. **Run `cargo test`** — integration tests pass alongside unit tests.

## Acceptance Criteria

- [ ] Smoke test demonstrates callback injection, invocation, and result propagation
- [ ] Smoke test demonstrates graceful no-op when callbacks are absent
- [ ] Test runs successfully with `cargo test`
- [ ] Test uses `#[tokio::test]` for async support

## Dependencies

- unit-tests

## Estimated Time

30 minutes
