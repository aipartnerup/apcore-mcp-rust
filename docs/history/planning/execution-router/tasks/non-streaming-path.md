# Task: non-streaming-path

## Goal

Implement the non-streaming execution path (`_handle_call_async`) that calls `executor.call_async()`, formats the result, and maps any errors via `ErrorMapper`. This is the primary execution path used when streaming is not available or not requested.

## Files Involved

- `src/server/router.rs` — Add `_handle_call_async` private method and `build_error_text` static method

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_call_async_success` — Mock executor returns `Value::Object`, result is formatted as JSON text content
   - `test_call_async_success_with_trace_id` — Result includes trace_id from context when available
   - `test_call_async_success_custom_formatter` — Custom output formatter is used for dict results
   - `test_call_async_error_mapped` — Mock executor returns error, ErrorMapper produces MCP error content
   - `test_call_async_error_text_includes_message` — Error content text includes the error message
   - `test_call_async_error_is_error_true` — Error result has `is_error = true`
   - `test_call_async_error_no_trace_id` — Error result has `trace_id = None`
   - `test_call_async_error_with_ai_guidance` — Error text includes AI guidance JSON when present
   - `test_build_error_text_simple` — Just message, no guidance fields
   - `test_build_error_text_with_retryable` — Message + `{"retryable": true}` appended
   - `test_build_error_text_with_all_guidance` — Message + JSON with all four guidance fields

2. **Implement `build_error_text` static method**:
   ```rust
   fn build_error_text(error_info: &Value) -> String {
       let text = error_info["message"].as_str().unwrap_or("Unknown error").to_string();
       let guidance_keys = ["retryable", "aiGuidance", "userFixable", "suggestion"];
       let guidance: serde_json::Map<String, Value> = guidance_keys
           .iter()
           .filter_map(|&k| error_info.get(k).map(|v| (k.to_string(), v.clone())))
           .collect();
       if guidance.is_empty() {
           text
       } else {
           format!("{text}\n\n{}", serde_json::to_string(&guidance).unwrap())
       }
   }
   ```

3. **Implement `_handle_call_async` private method**:
   ```rust
   async fn handle_call_async(
       &self,
       tool_name: &str,
       arguments: &Value,
       context: Option<&Context>,
   ) -> (Vec<ContentItem>, bool, Option<String>) {
       match self.executor.call_async(tool_name, arguments, context).await {
           Ok(result) => {
               let text = self.format_result(&result);
               let content = vec![ContentItem { content_type: "text".into(), data: Value::String(text) }];
               let trace_id = context.and_then(|c| c.trace_id().map(|s| s.to_string()));
               (content, false, trace_id)
           }
           Err(error) => {
               tracing::error!("handle_call error for {tool_name}: {error}");
               let error_info = ErrorMapper::to_mcp_error(&error);
               let text = Self::build_error_text(&error_info);
               (vec![ContentItem { content_type: "text".into(), data: Value::String(text) }], true, None)
           }
       }
   }
   ```

4. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Successful execution returns formatted text content with `is_error = false`
- [ ] Successful execution includes `trace_id` from context
- [ ] Custom output formatter is applied when configured
- [ ] Executor errors are caught and mapped via `ErrorMapper`
- [ ] Error results have `is_error = true` and `trace_id = None`
- [ ] `build_error_text` appends AI guidance JSON when guidance fields are present
- [ ] `build_error_text` returns plain message when no guidance fields exist
- [ ] All error paths log at `error` level via `tracing`
- [ ] All tests pass, clippy clean

## Dependencies

- context-construction
- output-formatting

## Estimated Time

1.5 hours
