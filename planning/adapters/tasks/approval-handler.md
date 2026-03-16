# Task: approval-handler

## Goal

Implement `ElicitationApprovalHandler` that bridges MCP elicitation to apcore's `ApprovalHandler` trait, sending approval prompts via an injected async elicit callback and mapping responses to `ApprovalResult`.

## Files Involved

- `src/adapters/approval.rs` — Full implementation replacing the current stub
- `src/helpers.rs` — Uses `ElicitAction`, `ElicitResult` types
- `tests/adapters/approval_test.rs` or inline `#[cfg(test)]` module

## Steps (TDD-first)

1. **Write unit tests first** (using mock elicit callbacks):
   - `test_request_approval_accepted` — Elicit returns `ElicitAction::Accept` -> `ApprovalResult { status: "approved" }`
   - `test_request_approval_declined` — Elicit returns `ElicitAction::Decline` -> `ApprovalResult { status: "rejected", reason: "User action: decline" }`
   - `test_request_approval_cancelled` — Elicit returns `ElicitAction::Cancel` -> `ApprovalResult { status: "rejected", reason: "User action: cancel" }`
   - `test_request_approval_no_callback` — Handler with no elicit callback -> `ApprovalResult { status: "rejected", reason: "No elicitation callback available" }`
   - `test_request_approval_callback_error` — Elicit callback returns error -> `ApprovalResult { status: "rejected", reason: "Elicitation request failed" }`
   - `test_request_approval_callback_none` — Elicit callback returns `None` -> `ApprovalResult { status: "rejected", reason: "Elicitation returned no response" }`
   - `test_check_approval_always_rejected` — `check_approval("any-id")` -> `ApprovalResult { status: "rejected", reason: "Phase B not supported via MCP elicitation" }`
   - `test_approval_message_format` — Verify the message sent to elicit contains module_id, description, and arguments

2. **Define the elicit callback type**:
   ```rust
   use std::future::Future;
   use std::pin::Pin;
   use std::sync::Arc;

   pub type ElicitCallback = Arc<
       dyn Fn(String, Option<Value>) -> Pin<Box<dyn Future<Output = Option<ElicitResult>> + Send>>
           + Send
           + Sync,
   >;
   ```

3. **Implement `ElicitationApprovalHandler`**:
   ```rust
   #[derive(Debug, Clone)]
   pub struct ElicitationApprovalHandler {
       elicit: Option<ElicitCallback>,
   }

   impl ElicitationApprovalHandler {
       pub fn new(elicit: Option<ElicitCallback>) -> Self {
           Self { elicit }
       }
   }
   ```
   Note: `Debug` impl will need a manual implementation since `ElicitCallback` is a trait object.

4. **Implement `ApprovalHandler` trait**:
   ```rust
   #[async_trait]
   impl ApprovalHandler for ElicitationApprovalHandler {
       async fn request_approval(&self, request: &ApprovalRequest) -> Result<ApprovalResult, ModuleError> {
           let elicit = match &self.elicit {
               Some(cb) => cb,
               None => return Ok(ApprovalResult {
                   status: "rejected".to_string(),
                   reason: Some("No elicitation callback available".to_string()),
                   ..Default::default()
               }),
           };

           let message = format!(
               "Approval required for tool: {}\n\n{}\n\nArguments: {}",
               request.module_id,
               request.description.as_deref().unwrap_or(""),
               request.arguments,
           );

           let result = match (elicit)(message, None).await {
               Some(r) => r,
               None => return Ok(ApprovalResult {
                   status: "rejected".to_string(),
                   reason: Some("Elicitation returned no response".to_string()),
                   ..Default::default()
               }),
           };

           match result.action {
               ElicitAction::Accept => Ok(ApprovalResult {
                   status: "approved".to_string(),
                   ..Default::default()
               }),
               other => Ok(ApprovalResult {
                   status: "rejected".to_string(),
                   reason: Some(format!("User action: {other:?}")),
                   ..Default::default()
               }),
           }
       }

       async fn check_approval(&self, _approval_id: &str) -> Result<ApprovalResult, ModuleError> {
           Ok(ApprovalResult {
               status: "rejected".to_string(),
               reason: Some("Phase B not supported via MCP elicitation".to_string()),
               ..Default::default()
           })
       }
   }
   ```

5. **Handle callback errors**:
   - If the elicit callback panics or the future returns an error, catch with `std::panic::AssertUnwindSafe` or wrap the call
   - Log the error via `tracing::debug!` and return rejected

6. **Implement manual `Debug`** for `ElicitationApprovalHandler`:
   ```rust
   impl std::fmt::Debug for ElicitationApprovalHandler {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           f.debug_struct("ElicitationApprovalHandler")
               .field("has_elicit", &self.elicit.is_some())
               .finish()
       }
   }
   ```

7. **Write mock callback helper** for tests:
   ```rust
   #[cfg(test)]
   fn mock_elicit(action: ElicitAction) -> ElicitCallback {
       Arc::new(move |_msg, _schema| {
           let action = action.clone();
           Box::pin(async move {
               Some(ElicitResult { action, content: None })
           })
       })
   }
   ```

8. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Implements `apcore::approval::ApprovalHandler` trait
- [ ] `request_approval` maps `ElicitAction::Accept` -> status "approved"
- [ ] `request_approval` maps `Decline`/`Cancel` -> status "rejected" with reason
- [ ] `request_approval` handles missing callback gracefully
- [ ] `request_approval` handles callback returning `None`
- [ ] `check_approval` always returns rejected with Phase B explanation
- [ ] Approval message includes module_id, description, and arguments
- [ ] Uses `tracing` for debug logging on failures
- [ ] All tests pass, clippy clean

## Dependencies

- adapter-setup
- annotations-mapper (conceptual dependency: approval flow checks `has_requires_approval` before invoking the handler, but this is wired at a higher level, not inside this module)

## Estimated Time

2 hours
