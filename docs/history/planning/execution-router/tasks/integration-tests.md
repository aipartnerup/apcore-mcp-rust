# Task: integration-tests

## Goal

Write end-to-end integration tests that exercise the full `ExecutionRouter` flow with mock executors, covering both streaming and non-streaming paths, validation, error handling, progress notifications, and elicitation. These tests verify that all components work together correctly.

## Files Involved

- `src/server/router.rs` — `#[cfg(test)]` integration test module (or `tests/server/router_integration.rs`)

## Steps (TDD-first)

1. **Build `MockExecutor` test helper**:
   ```rust
   struct MockExecutor {
       call_result: Mutex<Option<Result<Value, ExecutorError>>>,
       stream_chunks: Mutex<Option<Vec<Result<Value, ExecutorError>>>>,
       validate_result: Mutex<Option<ValidationResult>>,
       calls: Mutex<Vec<(String, Value)>>,
   }
   ```
   - Implements `Executor` trait
   - Records calls for assertion
   - Returns configurable results

2. **Build notification collector**:
   ```rust
   struct NotificationCollector {
       notifications: Mutex<Vec<Value>>,
   }
   ```
   - Provides a `SendNotificationFn` that captures all sent notifications

3. **Write integration tests**:
   - `test_e2e_simple_call` — Router calls executor, returns formatted JSON text
   - `test_e2e_call_with_custom_formatter` — Custom formatter transforms output
   - `test_e2e_call_error_mapped` — Executor error is mapped to MCP error content
   - `test_e2e_call_with_identity` — Identity is passed through to executor context
   - `test_e2e_streaming_three_chunks` — Three chunks streamed, three notifications sent, final result is deep-merged
   - `test_e2e_streaming_notification_content` — Verify notification JSON structure matches MCP spec
   - `test_e2e_streaming_error_mid_stream` — Error on second chunk returns error content
   - `test_e2e_streaming_fallback_no_support` — Executor without stream support falls to non-streaming
   - `test_e2e_validation_pass` — Validation passes, execution proceeds
   - `test_e2e_validation_fail` — Validation fails, execution skipped, error returned
   - `test_e2e_validation_disabled` — Validation disabled, invalid inputs still execute
   - `test_e2e_progress_callback_in_context` — Progress callback is accessible in context data
   - `test_e2e_elicit_callback_in_context` — Elicit callback is accessible in context data
   - `test_e2e_no_extra` — Call with no extra works (non-streaming, no callbacks)
   - `test_e2e_error_with_ai_guidance` — Error with AI guidance fields includes them in output

4. **Run full test suite** — ensure all pass. Run `cargo clippy`. Run `cargo test --all`.

## Acceptance Criteria

- [ ] MockExecutor supports configurable results for call_async, stream, and validate
- [ ] MockExecutor records calls for assertion
- [ ] NotificationCollector captures sent notifications
- [ ] All integration tests cover the happy path and error paths
- [ ] Streaming tests verify notification count and content
- [ ] Validation tests verify short-circuit behavior
- [ ] Identity propagation is verified
- [ ] Context callback injection is verified
- [ ] All tests pass, clippy clean
- [ ] No `#[ignore]` tests unless explicitly justified

## Dependencies

- handle-call-orchestrator

## Estimated Time

2 hours
