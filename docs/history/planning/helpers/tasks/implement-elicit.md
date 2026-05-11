# Task: Implement elicit

## Goal

Implement the `elicit` async function that extracts an elicitation callback from the execution context, invokes it, and returns `Option<ElicitResult>`, gracefully returning `None` when the callback is absent.

## Files Involved

- `src/helpers.rs` — replace the `todo!()` stub with working implementation

## Steps (TDD-first)

1. **Write tests first**:
   - Test that `elicit` with a mock context containing an `ElicitCallback` that returns `Some(ElicitResult { action: Accept, content: None })` returns the expected result.
   - Test that `elicit` with a mock context containing an `ElicitCallback` that returns `Some(ElicitResult { action: Decline, content: Some(json!({"reason": "..."})) })` preserves content.
   - Test that `elicit` with an empty context data map returns `None`.
   - Test that `elicit` with a missing key returns `None`.
   - Test that `elicit` with wrong-typed value returns `None` (downcast failure).
2. **Implement function**:
   ```rust
   pub async fn elicit(
       context: &Context,  // or appropriate type
       message: &str,
       requested_schema: Option<&Value>,
   ) -> Option<ElicitResult> {
       let data = context.data()?;  // or equivalent
       let any_callback = data.get(MCP_ELICIT_KEY)?;
       let callback = any_callback.downcast_ref::<ElicitCallback>().or_else(|| {
           tracing::debug!("elicit callback has unexpected type, skipping");
           None
       })?;
       callback(
           message.to_owned(),
           requested_schema.cloned(),
       ).await
   }
   ```
3. **Verify return type**: The function returns `Option<ElicitResult>` — `None` means elicitation is unavailable (no callback), which is distinct from the user choosing "cancel" (which is `Some(ElicitResult { action: Cancel, .. })`).
4. **Run tests** — all elicit tests pass.

## Acceptance Criteria

- [ ] `elicit` extracts callback from context data using `MCP_ELICIT_KEY`
- [ ] Callback is invoked with `(message, requested_schema)` when present
- [ ] Returns `Option<ElicitResult>` from the callback
- [ ] Returns `None` when context has no data
- [ ] Returns `None` when key is absent from data
- [ ] Returns `None` when stored value has wrong type (downcast fail)
- [ ] Debug-level tracing emitted on downcast failure
- [ ] Rustdoc comment on the public function

## Dependencies

- define-callback-types
- define-constants

## Estimated Time

45 minutes
