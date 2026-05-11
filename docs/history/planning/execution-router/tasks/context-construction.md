# Task: context-construction

## Goal

Implement per-call `Context` construction that injects MCP progress and elicitation callbacks into the context's data map, and passes identity from the auth layer. This mirrors the Python `Context.create(data=callbacks, identity=identity)` pattern.

## Files Involved

- `src/server/router.rs` — Add context building logic, callback types, and `SendNotificationFn` type alias

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_build_context_with_progress_callback` — When progress_token and send_notification are present, context data contains `MCP_PROGRESS_KEY`
   - `test_build_context_without_progress` — When progress_token is absent, context data does NOT contain `MCP_PROGRESS_KEY`
   - `test_build_context_with_elicit_callback` — When session is present, context data contains `MCP_ELICIT_KEY`
   - `test_build_context_without_session` — When session is absent, context data does NOT contain `MCP_ELICIT_KEY`
   - `test_build_context_with_identity` — Identity from extra is passed to Context
   - `test_build_context_without_identity` — Missing identity produces Context with no identity
   - `test_progress_callback_sends_notification` — The injected progress callback produces correct MCP notification JSON structure
   - `test_progress_callback_includes_message` — Progress callback includes message field when provided
   - `test_progress_callback_omits_message` — Progress callback omits message field when None
   - `test_elicit_callback_returns_result` — The injected elicit callback returns `ElicitResult` with action and content

2. **Define `SendNotificationFn` type alias**:
   ```rust
   pub type SendNotificationFn = Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>;
   ```

3. **Define `SessionHandle` trait** (for elicitation):
   ```rust
   #[async_trait]
   pub trait SessionHandle: Send + Sync {
       async fn elicit_form(
           &self,
           message: &str,
           requested_schema: &Value,
       ) -> Result<ElicitResult, Box<dyn std::error::Error + Send + Sync>>;
   }
   ```

4. **Define `CallExtra` struct** to replace raw `Option<&Value>`:
   ```rust
   pub struct CallExtra {
       pub progress_token: Option<ProgressToken>,
       pub send_notification: Option<SendNotificationFn>,
       pub session: Option<Arc<dyn SessionHandle>>,
       pub identity: Option<Value>,
   }

   pub enum ProgressToken {
       String(String),
       Integer(i64),
   }
   ```

5. **Implement `build_context` private method**:
   - Create empty `HashMap<String, Box<dyn Any + Send + Sync>>`
   - If progress_token + send_notification present: build progress closure, insert under `MCP_PROGRESS_KEY`
   - If session present: build elicit closure, insert under `MCP_ELICIT_KEY`
   - Call `Context::create(data, identity)`

6. **Implement progress closure**:
   - Captures `progress_token` (cloned) and `send_notification` (Arc cloned)
   - Builds `notifications/progress` JSON: `{ method, params: { progressToken, progress, total, message? } }`
   - Calls `send_notification` with the notification value

7. **Implement elicit closure**:
   - Captures `session` (Arc cloned)
   - Calls `session.elicit_form(message, requested_schema)`
   - Maps result to `{ action, content }` dict or returns `None` on error

8. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] `SendNotificationFn` type alias is defined
- [ ] `SessionHandle` trait is defined for elicitation
- [ ] `CallExtra` struct replaces raw `Value` for extra parameters
- [ ] `ProgressToken` enum supports both String and Integer variants
- [ ] Progress callback builds correct MCP notification JSON
- [ ] Progress callback includes `message` only when provided
- [ ] Elicit callback returns `ElicitResult` on success, `None` on failure
- [ ] Identity is passed through to Context
- [ ] Context data map uses keys from `helpers::MCP_PROGRESS_KEY` and `helpers::MCP_ELICIT_KEY`
- [ ] All captures are `Send + Sync`
- [ ] All tests pass, clippy clean

## Dependencies

- executor-trait (for Context type usage)

## Estimated Time

2 hours
