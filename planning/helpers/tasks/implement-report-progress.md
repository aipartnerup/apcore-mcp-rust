# Task: Implement report_progress

## Goal

Implement the `report_progress` async function that extracts a progress callback from the execution context and invokes it, gracefully no-oping when the callback is absent.

## Files Involved

- `src/helpers.rs` — replace the `todo!()` stub with working implementation

## Steps (TDD-first)

1. **Write tests first**:
   - Test that `report_progress` with a mock context containing a `ProgressCallback` invokes the callback with correct arguments.
   - Test that `report_progress` with an empty context data map returns `()` without panicking.
   - Test that `report_progress` with a context where the key exists but has the wrong type does not panic (graceful downcast failure).
2. **Determine context type**: Inspect the `apcore` crate's `Context` type to understand how `data` is accessed. If it uses `HashMap<String, Box<dyn Any + Send + Sync>>`, use `data.get(MCP_PROGRESS_KEY)` followed by `downcast_ref::<ProgressCallback>()`. If the context type differs, define a trait or adapter.
3. **Implement function**:
   ```rust
   pub async fn report_progress(
       context: &Context,  // or appropriate type
       progress: f64,
       total: Option<f64>,
       message: Option<&str>,
   ) {
       let Some(data) = context.data() else { return };
       let Some(any_callback) = data.get(MCP_PROGRESS_KEY) else { return };
       let Some(callback) = any_callback.downcast_ref::<ProgressCallback>() else {
           tracing::debug!("progress callback has unexpected type, skipping");
           return;
       };
       callback(progress, total, message.map(|s| s.to_owned())).await;
   }
   ```
4. **Update function signature**: Change `context: &Value` to the actual context type. Update the `message` parameter to `Option<&str>` to match the Python reference (where message is optional).
5. **Run tests** — all progress tests pass.

## Acceptance Criteria

- [ ] `report_progress` extracts callback from context data using `MCP_PROGRESS_KEY`
- [ ] Callback is invoked with `(progress, total, message)` when present
- [ ] Function returns `()` silently when context has no data
- [ ] Function returns `()` silently when key is absent from data
- [ ] Function returns `()` silently when stored value has wrong type (downcast fail)
- [ ] Function signature uses `Option<&str>` for `message` parameter
- [ ] Debug-level tracing emitted on downcast failure
- [ ] Rustdoc comment on the public function

## Dependencies

- define-callback-types
- define-constants

## Estimated Time

45 minutes
