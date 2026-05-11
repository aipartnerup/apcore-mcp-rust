# Task: handle-call-orchestrator

## Goal

Wire the public `handle_call` method that orchestrates the full execution flow: extract extras, build context, validate inputs, select streaming vs non-streaming path, and return the final result tuple. Also implement `ExecutionRouter::new` constructor.

## Files Involved

- `src/server/router.rs` — Implement `ExecutionRouter::new` and `handle_call`, update struct fields

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_new_default_formatter` — `ExecutionRouter::new` with `None` formatter creates valid router
   - `test_new_custom_formatter` — `ExecutionRouter::new` with custom formatter stores it
   - `test_new_validate_inputs_flag` — `validate_inputs` flag is stored and respected
   - `test_handle_call_non_streaming` — Without progress_token, routes to non-streaming path
   - `test_handle_call_streaming` — With progress_token + send_notification + executor.stream(), routes to streaming path
   - `test_handle_call_streaming_missing_send_notification` — With progress_token but no send_notification, falls back to non-streaming
   - `test_handle_call_streaming_executor_no_stream` — With progress_token + send_notification but executor.stream() returns None, falls back to non-streaming
   - `test_handle_call_validation_before_execution` — With validate_inputs=true, validation runs before execution
   - `test_handle_call_validation_failure_short_circuits` — Validation failure prevents execution
   - `test_handle_call_passes_identity` — Identity from CallExtra is passed to context
   - `test_handle_call_no_extra` — `None` extra works correctly (non-streaming, no callbacks)

2. **Update `ExecutionRouter` struct**:
   ```rust
   pub struct ExecutionRouter {
       executor: Box<dyn Executor>,
       validate_inputs: bool,
       output_formatter: Option<OutputFormatter>,
       error_mapper: ErrorMapper,
   }
   ```

3. **Implement `ExecutionRouter::new`**:
   ```rust
   pub fn new(
       executor: Box<dyn Executor>,
       validate_inputs: bool,
       output_formatter: Option<OutputFormatter>,
   ) -> Self {
       Self {
           executor,
           validate_inputs,
           output_formatter,
           error_mapper: ErrorMapper,
       }
   }
   ```

4. **Implement `handle_call`**:
   ```rust
   pub async fn handle_call(
       &self,
       tool_name: &str,
       arguments: &Value,
       extra: Option<CallExtra>,
   ) -> (Vec<ContentItem>, bool, Option<String>) {
       tracing::debug!("Executing tool call: {tool_name}");

       // Extract streaming helpers from extra
       let (progress_token, send_notification, session, identity) = match extra {
           Some(e) => (e.progress_token, e.send_notification, e.session, e.identity),
           None => (None, None, None, None),
       };

       // Build per-call context
       let context = self.build_context(
           progress_token.as_ref(),
           send_notification.as_ref(),
           session.as_ref(),
           identity.as_ref(),
       );

       // Pre-execution validation
       if self.validate_inputs {
           if let Some(validation) = self.executor.validate(tool_name, arguments, Some(&context)) {
               if !validation.valid {
                   let detail = Self::format_validation_errors(&validation.errors);
                   return (
                       vec![ContentItem {
                           content_type: "text".into(),
                           data: Value::String(format!("Validation failed: {detail}")),
                       }],
                       true,
                       None,
                   );
               }
           }
       }

       // Select execution path
       let can_stream = progress_token.is_some()
           && send_notification.is_some()
           && self.executor.stream(tool_name, arguments, Some(&context)).is_some();

       if can_stream {
           self.handle_stream(
               tool_name,
               arguments,
               progress_token.as_ref().unwrap(),
               send_notification.as_ref().unwrap(),
               Some(&context),
           ).await
       } else {
           self.handle_call_async(tool_name, arguments, Some(&context)).await
       }
   }
   ```

   Note: The `can_stream` check calls `executor.stream()` which creates the stream. To avoid double-creation, refactor to check for stream capability first (e.g., via a separate `supports_stream` method or by consuming the stream directly). This is a refinement to address during implementation.

5. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] `ExecutionRouter::new` accepts executor, validate_inputs, and optional output_formatter
- [ ] `handle_call` extracts progress_token, send_notification, session, identity from CallExtra
- [ ] `handle_call` builds per-call Context with callbacks and identity
- [ ] Validation runs before execution when `validate_inputs = true`
- [ ] Validation failure short-circuits execution
- [ ] Streaming path is selected when executor supports stream AND progress_token + send_notification present
- [ ] Falls back to non-streaming when any streaming prerequisite is missing
- [ ] `None` extra is handled gracefully (non-streaming, no callbacks)
- [ ] Debug-level log emitted at start of each handle_call
- [ ] All tests pass, clippy clean

## Dependencies

- non-streaming-path
- streaming-path
- input-validation

## Estimated Time

1.5 hours
