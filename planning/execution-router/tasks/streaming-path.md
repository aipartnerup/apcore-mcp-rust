# Task: streaming-path

## Goal

Implement the streaming execution path (`_handle_stream`) that iterates an async stream from the executor, sends progress notifications for each chunk, accumulates results via deep merge, and returns the final merged result. This mirrors the Python `_handle_stream` method.

## Files Involved

- `src/server/router.rs` — Add `_handle_stream` private method

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_stream_single_chunk` — One chunk streamed, one progress notification sent, result matches chunk
   - `test_stream_multiple_chunks_merged` — Three chunks deep-merged, three progress notifications sent
   - `test_stream_empty` — Zero chunks, result is empty object, no notifications sent
   - `test_stream_progress_notification_structure` — Notification has `method: "notifications/progress"`, `params.progressToken`, `params.progress` (1-indexed), `params.total: null`, `params.message` (JSON of chunk)
   - `test_stream_progress_token_string` — String progress token is passed through
   - `test_stream_progress_token_integer` — Integer progress token is passed through
   - `test_stream_accumulates_nested` — Nested objects are deep-merged across chunks
   - `test_stream_error_mid_stream` — Error on chunk 2 of 3 maps to error result via ErrorMapper
   - `test_stream_error_is_error_true` — Stream error result has `is_error = true`
   - `test_stream_result_formatted` — Final accumulated result is formatted via output_formatter
   - `test_stream_result_has_trace_id` — Successful stream returns trace_id from context

2. **Implement `_handle_stream` private method**:
   ```rust
   async fn handle_stream(
       &self,
       tool_name: &str,
       arguments: &Value,
       progress_token: &ProgressToken,
       send_notification: &SendNotificationFn,
       context: Option<&Context>,
   ) -> (Vec<ContentItem>, bool, Option<String>) {
       let mut accumulated = Value::Object(serde_json::Map::new());
       let mut chunk_index: usize = 0;

       let stream = match self.executor.stream(tool_name, arguments, context) {
           Some(s) => s,
           None => {
               // Fallback to non-streaming if stream() returns None
               return self.handle_call_async(tool_name, arguments, context).await;
           }
       };

       tokio::pin!(stream);

       loop {
           match stream.next().await {
               Some(Ok(chunk)) => {
                   // Build and send progress notification
                   let notification = serde_json::json!({
                       "method": "notifications/progress",
                       "params": {
                           "progressToken": progress_token_to_value(progress_token),
                           "progress": chunk_index + 1,
                           "total": null,
                           "message": serde_json::to_string(&chunk).unwrap_or_default(),
                       }
                   });
                   if let Err(e) = send_notification(notification).await {
                       tracing::debug!("Failed to send progress notification: {e}");
                   }

                   accumulated = deep_merge(&accumulated, &chunk, 0);
                   chunk_index += 1;
               }
               Some(Err(error)) => {
                   tracing::error!("handle_call stream error for {tool_name}: {error}");
                   let error_info = ErrorMapper::to_mcp_error(&error);
                   let text = Self::build_error_text(&error_info);
                   return (
                       vec![ContentItem { content_type: "text".into(), data: Value::String(text) }],
                       true,
                       None,
                   );
               }
               None => break, // stream ended
           }
       }

       let text = self.format_result(&accumulated);
       let content = vec![ContentItem { content_type: "text".into(), data: Value::String(text) }];
       let trace_id = context.and_then(|c| c.trace_id().map(|s| s.to_string()));
       (content, false, trace_id)
   }
   ```

3. **Implement `progress_token_to_value` helper**:
   ```rust
   fn progress_token_to_value(token: &ProgressToken) -> Value {
       match token {
           ProgressToken::String(s) => Value::String(s.clone()),
           ProgressToken::Integer(i) => Value::Number((*i).into()),
       }
   }
   ```

4. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Iterates async stream using `StreamExt::next()`
- [ ] Sends `notifications/progress` for each chunk with 1-indexed progress counter
- [ ] Progress notification includes chunk as JSON string in `message` field
- [ ] Accumulates chunks via `deep_merge` starting from empty object
- [ ] Returns final accumulated result formatted via output_formatter
- [ ] Returns trace_id from context on success
- [ ] Catches stream errors and maps via ErrorMapper
- [ ] Falls back to non-streaming path if `stream()` returns `None`
- [ ] Handles both string and integer progress tokens
- [ ] All tests pass, clippy clean

## Dependencies

- context-construction
- output-formatting
- deep-merge

## Estimated Time

2 hours
