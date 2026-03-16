//! ExecutionRouter — routes MCP tool calls to the apcore executor.
//!
//! Handles argument validation, execution dispatch, and output formatting.

#![allow(unused)]

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_stream::Stream;

use crate::helpers::{ElicitResult, MCP_ELICIT_KEY, MCP_PROGRESS_KEY};

// ---------------------------------------------------------------------------
// Task 1: Executor trait
// ---------------------------------------------------------------------------

/// Errors returned by [`Executor`] methods.
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    /// A module execution failed with a structured error code.
    #[error("{message}")]
    Execution {
        code: String,
        message: String,
        details: Option<Value>,
    },
    /// Input validation failed.
    #[error("validation failed: {0}")]
    Validation(String),
    /// Any other error.
    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A single field-level validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// The field that failed validation (if applicable).
    pub field: Option<String>,
    /// Human-readable error message.
    pub message: String,
    /// Nested validation errors (e.g. for nested objects).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ValidationError>,
}

/// Result of validating module inputs.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the inputs are valid.
    pub valid: bool,
    /// Field-level errors (empty when `valid` is true).
    pub errors: Vec<ValidationError>,
}

/// Abstraction over the apcore execution pipeline.
///
/// Provides async `call_async`, optional `stream`, and optional `validate`
/// methods. Object-safe so it can be used as `Box<dyn Executor>`.
#[async_trait]
pub trait Executor: Send + Sync {
    /// Execute a module by ID and return the result.
    async fn call_async(
        &self,
        module_id: &str,
        inputs: &Value,
        context: Option<&Value>,
    ) -> Result<Value, ExecutorError>;

    /// Return a stream of result chunks for a module, or `None` if
    /// the executor does not support streaming.
    fn stream(
        &self,
        _module_id: &str,
        _inputs: &Value,
        _context: Option<&Value>,
    ) -> Option<Pin<Box<dyn Stream<Item = Result<Value, ExecutorError>> + Send>>> {
        None
    }

    /// Validate inputs for a module, or `None` if the executor does
    /// not support pre-execution validation.
    fn validate(
        &self,
        _module_id: &str,
        _inputs: &Value,
        _context: Option<&Value>,
    ) -> Option<ValidationResult> {
        None
    }
}

// ---------------------------------------------------------------------------
// Task 2: deep_merge
// ---------------------------------------------------------------------------

/// Maximum recursion depth for [`deep_merge`].
const DEEP_MERGE_MAX_DEPTH: usize = 32;

/// Recursively merge `overlay` into `base`, capped at [`DEEP_MERGE_MAX_DEPTH`].
///
/// When both sides have an object for the same key, the merge recurses.
/// All other types (arrays, scalars, null) are overwritten by `overlay`.
pub(crate) fn deep_merge(base: &Value, overlay: &Value, depth: usize) -> Value {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            if depth >= DEEP_MERGE_MAX_DEPTH {
                // Flat merge: overlay keys win, no further recursion.
                let mut merged = base_map.clone();
                for (k, v) in overlay_map {
                    merged.insert(k.clone(), v.clone());
                }
                return Value::Object(merged);
            }
            let mut merged = base_map.clone();
            for (k, v) in overlay_map {
                let new_val = match merged.get(k) {
                    Some(existing_val) => deep_merge(existing_val, v, depth + 1),
                    None => v.clone(),
                };
                merged.insert(k.clone(), new_val);
            }
            Value::Object(merged)
        }
        _ => overlay.clone(),
    }
}

// ---------------------------------------------------------------------------
// Task 3: Output formatting
// ---------------------------------------------------------------------------

/// A custom formatter that converts execution results into text.
///
/// Only called for `Value::Object` results. Must be `Send + Sync` so it
/// can be shared across async tasks.
pub type OutputFormatter =
    Box<dyn Fn(&Value) -> Result<String, Box<dyn std::error::Error>> + Send + Sync>;

// ---------------------------------------------------------------------------
// Task 4: Context construction types
// ---------------------------------------------------------------------------

/// Async function for sending MCP notifications (e.g. progress).
pub type SendNotificationFn = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Handle to the MCP session for elicitation requests.
#[async_trait]
pub trait SessionHandle: Send + Sync {
    /// Send an elicitation form to the client and return the result.
    async fn elicit_form(
        &self,
        message: &str,
        requested_schema: &Value,
    ) -> Result<ElicitResult, Box<dyn std::error::Error + Send + Sync>>;
}

/// Progress token — either a string or integer as per MCP spec.
#[derive(Debug, Clone)]
pub enum ProgressToken {
    /// A string progress token.
    String(String),
    /// An integer progress token.
    Integer(i64),
}

/// Structured extra parameters extracted from the MCP call metadata.
pub struct CallExtra {
    /// Progress token for streaming notifications.
    pub progress_token: Option<ProgressToken>,
    /// Notification sender for progress updates.
    pub send_notification: Option<SendNotificationFn>,
    /// Session handle for elicitation.
    pub session: Option<Arc<dyn SessionHandle>>,
    /// Identity from auth middleware.
    pub identity: Option<Value>,
}

/// Convert a [`ProgressToken`] to a JSON [`Value`].
fn progress_token_to_value(token: &ProgressToken) -> Value {
    match token {
        ProgressToken::String(s) => Value::String(s.clone()),
        ProgressToken::Integer(i) => serde_json::json!(*i),
    }
}

// ---------------------------------------------------------------------------
// ExecutionRouter
// ---------------------------------------------------------------------------

/// A single piece of MCP content returned from a tool execution.
#[derive(Debug, Clone)]
pub struct ContentItem {
    /// Content type (e.g. "text", "image", "resource").
    pub content_type: String,
    /// The content payload.
    pub data: Value,
}

/// Routes incoming MCP tool calls to the underlying apcore executor.
pub struct ExecutionRouter {
    executor: Option<Box<dyn Executor>>,
    validate_inputs: bool,
    output_formatter: Option<OutputFormatter>,
}

impl ExecutionRouter {
    /// Create a stub router for testing.
    ///
    /// The stub router does not execute any modules. Its `handle_call`
    /// method will panic if invoked (it is only used to verify handler
    /// wiring, not actual execution).
    pub fn stub() -> Self {
        Self {
            executor: None,
            validate_inputs: false,
            output_formatter: None,
        }
    }

    /// Create a new router.
    ///
    /// # Arguments
    /// * `executor` - The executor to delegate tool calls to.
    /// * `validate_inputs` - Whether to validate tool inputs against their schema.
    /// * `output_formatter` - Optional custom output formatter.
    pub fn new(
        executor: Box<dyn Executor>,
        validate_inputs: bool,
        output_formatter: Option<OutputFormatter>,
    ) -> Self {
        Self {
            executor: Some(executor),
            validate_inputs,
            output_formatter,
        }
    }

    /// Format an execution result into text for LLM consumption.
    ///
    /// Uses the configured `output_formatter` if set, otherwise falls back
    /// to `serde_json::to_string`. The custom formatter is only applied to
    /// `Value::Object` results.
    fn format_result(&self, result: &Value) -> String {
        if let Some(ref formatter) = self.output_formatter {
            if result.is_object() {
                match formatter(result) {
                    Ok(text) => return text,
                    Err(e) => {
                        tracing::debug!("output_formatter failed, falling back to json: {e}");
                    }
                }
            }
        }
        serde_json::to_string(result).unwrap_or_else(|_| "null".to_string())
    }

    // -----------------------------------------------------------------------
    // Task 7: Input validation
    // -----------------------------------------------------------------------

    /// Format validation errors into a human-readable string.
    ///
    /// Field errors are formatted as `"field: message"` and joined by `"; "`.
    /// Nested errors (sub-errors) are flattened. Errors without a field fall
    /// back to the message text alone.
    fn format_validation_errors(errors: &[ValidationError]) -> String {
        let parts: Vec<String> = errors
            .iter()
            .flat_map(|e| {
                if !e.errors.is_empty() {
                    e.errors
                        .iter()
                        .map(|sub| {
                            format!(
                                "{}: {}",
                                sub.field.as_deref().unwrap_or("?"),
                                &sub.message
                            )
                        })
                        .collect::<Vec<_>>()
                } else if let Some(ref field) = e.field {
                    vec![format!("{field}: {}", &e.message)]
                } else {
                    vec![e.message.clone()]
                }
            })
            .collect();
        parts.join("; ")
    }

    // -----------------------------------------------------------------------
    // Task 8: handle_call orchestrator
    // -----------------------------------------------------------------------

    /// Handle an MCP tool call.
    ///
    /// Orchestrates the full execution flow:
    /// 1. Extract extras and build per-call context
    /// 2. Pre-execution input validation (when `validate_inputs` is true)
    /// 3. Select streaming vs non-streaming execution path
    /// 4. Return `(content_items, is_error, trace_id)`
    ///
    /// # Arguments
    /// * `tool_name` - The normalized MCP tool name.
    /// * `arguments` - The tool arguments as a JSON object.
    /// * `extra` - Additional MCP call metadata.
    ///
    /// # Returns
    /// A tuple of (content items, is_error flag, optional trace_id).
    pub async fn handle_call(
        &self,
        tool_name: &str,
        arguments: &Value,
        extra: Option<&Value>,
    ) -> (Vec<ContentItem>, bool, Option<String>) {
        tracing::debug!("Executing tool call: {tool_name}");

        // Extract streaming helpers and identity from extra
        let (progress_token, send_notification, session, identity) =
            Self::extract_extra(extra);

        // Build per-call context
        let call_extra = CallExtra {
            progress_token,
            send_notification,
            session,
            identity,
        };
        let (context_value, _context_data) = Self::build_context(&call_extra);

        // Re-extract after building context (we moved them into CallExtra)
        let (progress_token, send_notification, _, _) =
            Self::extract_extra(extra);

        // Pre-execution validation
        if self.validate_inputs {
            if let Some(ref executor) = self.executor {
                match executor.validate(tool_name, arguments, Some(&context_value)) {
                    Some(validation) if !validation.valid => {
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
                    Some(_) => { /* valid, continue */ }
                    None => { /* executor doesn't support validate, skip */ }
                }
            }
        }

        // Select execution path: try streaming if both prerequisites present;
        // handle_stream will fall back to non-streaming if executor doesn't
        // support streaming.
        if let (Some(ref pt), Some(ref sn)) = (progress_token, send_notification) {
            self.handle_stream(tool_name, arguments, pt, sn, Some(&context_value))
                .await
        } else {
            self.handle_call_async(tool_name, arguments, Some(&context_value))
                .await
        }
    }

    /// Extract progress_token, send_notification, session, and identity from
    /// the extra `Value`. Returns `(None, None, None, None)` when extra is
    /// `None`.
    fn extract_extra(
        extra: Option<&Value>,
    ) -> (
        Option<ProgressToken>,
        Option<SendNotificationFn>,
        Option<Arc<dyn SessionHandle>>,
        Option<Value>,
    ) {
        // In the current integration the factory passes a plain JSON Value
        // which does not carry callbacks. Real callbacks would come from a
        // typed CallExtra.  For now we extract identity from JSON.
        let identity = extra
            .and_then(|v| v.get("identity"))
            .cloned();

        let progress_token = extra
            .and_then(|v| v.get("progress_token"))
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(ProgressToken::String(s.to_string()))
                } else if let Some(i) = v.as_i64() {
                    Some(ProgressToken::Integer(i))
                } else {
                    None
                }
            });

        // send_notification and session cannot be extracted from plain JSON
        (progress_token, None, None, identity)
    }

    /// Handle an MCP tool call with a typed `CallExtra`.
    ///
    /// This is the full-featured entry point that supports streaming,
    /// elicitation, and progress callbacks via the `CallExtra` struct.
    pub async fn handle_call_with_extra(
        &self,
        tool_name: &str,
        arguments: &Value,
        extra: Option<CallExtra>,
    ) -> (Vec<ContentItem>, bool, Option<String>) {
        tracing::debug!("Executing tool call: {tool_name}");

        let extra = extra.unwrap_or_else(|| CallExtra {
            progress_token: None,
            send_notification: None,
            session: None,
            identity: None,
        });

        // Build per-call context
        let (context_value, _context_data) = Self::build_context(&extra);

        // Pre-execution validation
        if self.validate_inputs {
            if let Some(ref executor) = self.executor {
                match executor.validate(tool_name, arguments, Some(&context_value)) {
                    Some(validation) if !validation.valid => {
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
                    Some(_) => { /* valid, continue */ }
                    None => { /* executor doesn't support validate, skip */ }
                }
            }
        }

        // Select execution path: try streaming if both prerequisites present;
        // handle_stream will fall back to non-streaming if executor doesn't
        // support streaming.
        if let (Some(ref pt), Some(ref sn)) = (extra.progress_token, extra.send_notification) {
            self.handle_stream(tool_name, arguments, pt, sn, Some(&context_value))
                .await
        } else {
            self.handle_call_async(tool_name, arguments, Some(&context_value))
                .await
        }
    }

    // -----------------------------------------------------------------------
    // Task 4: Context construction
    // -----------------------------------------------------------------------

    /// Build execution context with MCP callbacks and identity.
    ///
    /// Constructs a JSON context value and, if callbacks are present,
    /// stores them under `MCP_PROGRESS_KEY` / `MCP_ELICIT_KEY` in the
    /// returned data map.
    ///
    /// Returns `(context_value, context_data)` where `context_data` holds
    /// the callback objects that cannot be serialized into JSON.
    fn build_context(
        extra: &CallExtra,
    ) -> (Value, HashMap<String, Box<dyn std::any::Any + Send + Sync>>) {
        let mut data: HashMap<String, Box<dyn std::any::Any + Send + Sync>> = HashMap::new();
        let mut context_obj = serde_json::Map::new();

        // Inject progress callback
        if let (Some(ref token), Some(ref send_notification)) =
            (&extra.progress_token, &extra.send_notification)
        {
            let token = token.clone();
            let sn = Arc::clone(send_notification);
            let progress_cb: crate::helpers::ProgressCallback =
                Box::new(move |progress, total, message| {
                    let token_val = progress_token_to_value(&token);
                    let sn = Arc::clone(&sn);
                    Box::pin(async move {
                        let mut params = serde_json::Map::new();
                        params.insert("progressToken".to_string(), token_val);
                        params.insert(
                            "progress".to_string(),
                            serde_json::json!(progress),
                        );
                        params.insert(
                            "total".to_string(),
                            total.map(|t| serde_json::json!(t)).unwrap_or(serde_json::json!(0)),
                        );
                        if let Some(msg) = message {
                            params.insert("message".to_string(), Value::String(msg));
                        }
                        let notification = serde_json::json!({
                            "method": "notifications/progress",
                            "params": Value::Object(params),
                        });
                        if let Err(e) = sn(notification).await {
                            tracing::debug!("Failed to send progress notification: {e}");
                        }
                    })
                });
            data.insert(
                MCP_PROGRESS_KEY.to_string(),
                Box::new(progress_cb),
            );
        }

        // Inject elicit callback
        if let Some(ref session) = extra.session {
            let session = Arc::clone(session);
            let elicit_cb: crate::helpers::ElicitCallback =
                Box::new(move |message, requested_schema| {
                    let session = Arc::clone(&session);
                    Box::pin(async move {
                        let schema = requested_schema.unwrap_or(serde_json::json!({}));
                        match session.elicit_form(&message, &schema).await {
                            Ok(result) => Some(result),
                            Err(e) => {
                                tracing::debug!("Elicitation request failed: {e}");
                                None
                            }
                        }
                    })
                });
            data.insert(MCP_ELICIT_KEY.to_string(), Box::new(elicit_cb));
        }

        // Set identity
        if let Some(ref identity) = extra.identity {
            context_obj.insert("identity".to_string(), identity.clone());
        }

        // Set trace_id
        context_obj.insert(
            "trace_id".to_string(),
            Value::String(uuid::Uuid::new_v4().to_string()),
        );

        (Value::Object(context_obj), data)
    }

    // -----------------------------------------------------------------------
    // Task 5: Non-streaming path
    // -----------------------------------------------------------------------

    /// Build error text from an `ExecutorError`, appending AI guidance when
    /// available.
    fn build_error_text(error: &ExecutorError) -> String {
        match error {
            ExecutorError::Execution {
                code,
                message,
                details,
            } => {
                let mut text = message.clone();
                // Check details for guidance fields
                if let Some(ref d) = details {
                    let guidance_keys = ["retryable", "aiGuidance", "userFixable", "suggestion"];
                    let guidance: serde_json::Map<String, Value> = guidance_keys
                        .iter()
                        .filter_map(|&k| d.get(k).map(|v| (k.to_string(), v.clone())))
                        .collect();
                    if !guidance.is_empty() {
                        text = format!(
                            "{text}\n\n{}",
                            serde_json::to_string(&guidance).unwrap_or_default()
                        );
                    }
                }
                text
            }
            ExecutorError::Validation(msg) => format!("Validation failed: {msg}"),
            ExecutorError::Other(e) => e.to_string(),
        }
    }

    /// Non-streaming execution via executor.call_async().
    async fn handle_call_async(
        &self,
        tool_name: &str,
        arguments: &Value,
        context: Option<&Value>,
    ) -> (Vec<ContentItem>, bool, Option<String>) {
        let executor = match &self.executor {
            Some(e) => e,
            None => {
                return (
                    vec![ContentItem {
                        content_type: "text".into(),
                        data: Value::String("No executor configured".into()),
                    }],
                    true,
                    None,
                );
            }
        };

        match executor.call_async(tool_name, arguments, context).await {
            Ok(result) => {
                let text = self.format_result(&result);
                let content = vec![ContentItem {
                    content_type: "text".into(),
                    data: Value::String(text),
                }];
                let trace_id = context
                    .and_then(|c| c.get("trace_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                (content, false, trace_id)
            }
            Err(error) => {
                tracing::error!("handle_call error for {tool_name}: {error}");
                let text = Self::build_error_text(&error);
                (
                    vec![ContentItem {
                        content_type: "text".into(),
                        data: Value::String(text),
                    }],
                    true,
                    None,
                )
            }
        }
    }

    // -----------------------------------------------------------------------
    // Task 6: Streaming path
    // -----------------------------------------------------------------------

    /// Streaming execution via executor.stream().
    ///
    /// Iterates the async stream, sends progress notifications for each
    /// chunk, accumulates results via deep merge, and returns the final
    /// result. Falls back to non-streaming if the executor returns `None`.
    async fn handle_stream(
        &self,
        tool_name: &str,
        arguments: &Value,
        progress_token: &ProgressToken,
        send_notification: &SendNotificationFn,
        context: Option<&Value>,
    ) -> (Vec<ContentItem>, bool, Option<String>) {
        use tokio_stream::StreamExt;

        let executor = match &self.executor {
            Some(e) => e,
            None => {
                return (
                    vec![ContentItem {
                        content_type: "text".into(),
                        data: Value::String("No executor configured".into()),
                    }],
                    true,
                    None,
                );
            }
        };

        let stream = match executor.stream(tool_name, arguments, context) {
            Some(s) => s,
            None => {
                // Fallback to non-streaming
                return self
                    .handle_call_async(tool_name, arguments, context)
                    .await;
            }
        };

        tokio::pin!(stream);

        let mut accumulated = Value::Object(serde_json::Map::new());
        let mut chunk_index: usize = 0;

        loop {
            match stream.next().await {
                Some(Ok(chunk)) => {
                    // Send progress notification
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
                    let text = Self::build_error_text(&error);
                    return (
                        vec![ContentItem {
                            content_type: "text".into(),
                            data: Value::String(text),
                        }],
                        true,
                        None,
                    );
                }
                None => break,
            }
        }

        let text = self.format_result(&accumulated);
        let content = vec![ContentItem {
            content_type: "text".into(),
            data: Value::String(text),
        }];
        let trace_id = context
            .and_then(|c| c.get("trace_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        (content, false, trace_id)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    // ---- Task 1: Executor trait tests ----

    /// A mock executor for testing. Returns `inputs` as the result.
    struct MockExecutor;

    #[async_trait]
    impl Executor for MockExecutor {
        async fn call_async(
            &self,
            module_id: &str,
            inputs: &Value,
            _context: Option<&Value>,
        ) -> Result<Value, ExecutorError> {
            Ok(json!({ "module": module_id, "echo": inputs }))
        }
    }

    /// A mock executor that supports streaming and validation.
    struct FullMockExecutor;

    #[async_trait]
    impl Executor for FullMockExecutor {
        async fn call_async(
            &self,
            _module_id: &str,
            inputs: &Value,
            _context: Option<&Value>,
        ) -> Result<Value, ExecutorError> {
            Ok(inputs.clone())
        }

        fn stream(
            &self,
            _module_id: &str,
            _inputs: &Value,
            _context: Option<&Value>,
        ) -> Option<Pin<Box<dyn Stream<Item = Result<Value, ExecutorError>> + Send>>> {
            let chunks = vec![
                Ok(json!({"a": 1})),
                Ok(json!({"b": 2})),
            ];
            Some(Box::pin(tokio_stream::iter(chunks)))
        }

        fn validate(
            &self,
            _module_id: &str,
            inputs: &Value,
            _context: Option<&Value>,
        ) -> Option<ValidationResult> {
            // If inputs has "invalid" key, return invalid
            if inputs.get("invalid").is_some() {
                Some(ValidationResult {
                    valid: false,
                    errors: vec![ValidationError {
                        field: Some("invalid".to_string()),
                        message: "field is not allowed".to_string(),
                        errors: vec![],
                    }],
                })
            } else {
                Some(ValidationResult {
                    valid: true,
                    errors: vec![],
                })
            }
        }
    }

    #[tokio::test]
    async fn test_mock_executor_call_async() {
        let executor = MockExecutor;
        let result = executor
            .call_async("test.module", &json!({"key": "value"}), None)
            .await
            .unwrap();
        assert_eq!(result["module"], "test.module");
        assert_eq!(result["echo"]["key"], "value");
    }

    #[test]
    fn test_mock_executor_stream_none() {
        let executor = MockExecutor;
        let stream = executor.stream("test.module", &json!({}), None);
        assert!(stream.is_none());
    }

    #[test]
    fn test_mock_executor_stream_some() {
        let executor = FullMockExecutor;
        let stream = executor.stream("test.module", &json!({}), None);
        assert!(stream.is_some());
    }

    #[tokio::test]
    async fn test_mock_executor_stream_yields_chunks() {
        use tokio_stream::StreamExt;

        let executor = FullMockExecutor;
        let mut stream = executor.stream("test.module", &json!({}), None).unwrap();

        let chunk1 = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk1, json!({"a": 1}));

        let chunk2 = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk2, json!({"b": 2}));

        assert!(stream.next().await.is_none());
    }

    #[test]
    fn test_mock_executor_validate_none() {
        let executor = MockExecutor;
        let result = executor.validate("test.module", &json!({}), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_mock_executor_validate_valid() {
        let executor = FullMockExecutor;
        let result = executor
            .validate("test.module", &json!({"name": "ok"}), None)
            .unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_mock_executor_validate_invalid() {
        let executor = FullMockExecutor;
        let result = executor
            .validate("test.module", &json!({"invalid": true}), None)
            .unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, Some("invalid".to_string()));
    }

    #[test]
    fn test_executor_is_object_safe() {
        // Compile-time test: Box<dyn Executor> must compile.
        fn _assert_object_safe(_e: Box<dyn Executor>) {}
    }

    // ---- Task 2: deep_merge tests ----

    #[test]
    fn test_deep_merge_flat_objects() {
        let base = json!({"a": 1, "b": 2});
        let overlay = json!({"b": 3, "c": 4});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"a": 1, "b": 3, "c": 4}));
    }

    #[test]
    fn test_deep_merge_nested_objects() {
        let base = json!({"a": {"x": 1, "y": 2}});
        let overlay = json!({"a": {"y": 3, "z": 4}});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"a": {"x": 1, "y": 3, "z": 4}}));
    }

    #[test]
    fn test_deep_merge_overlay_overwrites_non_object() {
        let base = json!({"a": "string"});
        let overlay = json!({"a": {"nested": true}});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"a": {"nested": true}}));
    }

    #[test]
    fn test_deep_merge_base_dict_overlay_scalar() {
        let base = json!({"a": {"nested": true}});
        let overlay = json!({"a": "scalar"});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"a": "scalar"}));
    }

    #[test]
    fn test_deep_merge_empty_base() {
        let base = json!({});
        let overlay = json!({"a": 1});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"a": 1}));
    }

    #[test]
    fn test_deep_merge_empty_overlay() {
        let base = json!({"a": 1});
        let overlay = json!({});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"a": 1}));
    }

    #[test]
    fn test_deep_merge_both_empty() {
        let base = json!({});
        let overlay = json!({});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_deep_merge_depth_cap() {
        // At depth 32, should do flat merge (overlay wins for conflicting keys)
        let base = json!({"a": {"inner": "base"}});
        let overlay = json!({"a": {"inner": "overlay", "extra": true}});
        let result = deep_merge(&base, &overlay, DEEP_MERGE_MAX_DEPTH);
        // At max depth, the entire overlay value for "a" wins (flat merge of top-level keys)
        assert_eq!(result["a"]["inner"], "overlay");
        assert_eq!(result["a"]["extra"], true);
    }

    #[test]
    fn test_deep_merge_depth_31_still_recurses() {
        // At depth 31 (one below max), recursion still happens
        let base = json!({"a": {"x": 1, "y": 2}});
        let overlay = json!({"a": {"y": 3}});
        let result = deep_merge(&base, &overlay, 31);
        // Should recurse into "a" and merge
        assert_eq!(result, json!({"a": {"x": 1, "y": 3}}));
    }

    #[test]
    fn test_deep_merge_non_object_inputs() {
        // When base is not an object, overlay wins
        let result = deep_merge(&json!("string"), &json!(42), 0);
        assert_eq!(result, json!(42));

        let result = deep_merge(&json!([1, 2]), &json!([3, 4]), 0);
        assert_eq!(result, json!([3, 4]));
    }

    #[test]
    fn test_deep_merge_three_levels_deep() {
        let base = json!({"a": {"b": {"c": 1, "d": 2}}});
        let overlay = json!({"a": {"b": {"d": 3, "e": 4}}});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"a": {"b": {"c": 1, "d": 3, "e": 4}}}));
    }

    #[test]
    fn test_deep_merge_array_not_merged() {
        // Arrays are overwritten, not concatenated
        let base = json!({"items": [1, 2, 3]});
        let overlay = json!({"items": [4, 5]});
        let result = deep_merge(&base, &overlay, 0);
        assert_eq!(result, json!({"items": [4, 5]}));
    }

    // ---- Task 3: Output formatting tests ----

    #[test]
    fn test_format_result_default_json() {
        let router = ExecutionRouter::stub();
        let result = router.format_result(&json!({"key": "value"}));
        assert_eq!(result, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_format_result_default_string() {
        let router = ExecutionRouter::stub();
        let result = router.format_result(&json!("hello"));
        assert_eq!(result, r#""hello""#);
    }

    #[test]
    fn test_format_result_default_number() {
        let router = ExecutionRouter::stub();
        let result = router.format_result(&json!(42));
        assert_eq!(result, "42");
    }

    #[test]
    fn test_format_result_default_null() {
        let router = ExecutionRouter::stub();
        let result = router.format_result(&Value::Null);
        assert_eq!(result, "null");
    }

    #[test]
    fn test_format_result_custom_formatter() {
        let formatter: OutputFormatter = Box::new(|val| {
            let obj = val.as_object().unwrap();
            Ok(format!("custom: {} keys", obj.len()))
        });
        let router = ExecutionRouter {
            executor: None,
            validate_inputs: false,
            output_formatter: Some(formatter),
        };
        let result = router.format_result(&json!({"a": 1, "b": 2}));
        assert_eq!(result, "custom: 2 keys");
    }

    #[test]
    fn test_format_result_custom_formatter_non_object_ignored() {
        let formatter: OutputFormatter = Box::new(|_val| {
            Ok("should not be called".to_string())
        });
        let router = ExecutionRouter {
            executor: None,
            validate_inputs: false,
            output_formatter: Some(formatter),
        };
        // Non-object values should fall back to JSON
        assert_eq!(router.format_result(&json!("string")), r#""string""#);
        assert_eq!(router.format_result(&json!(123)), "123");
        assert_eq!(router.format_result(&json!([1, 2])), "[1,2]");
    }

    #[test]
    fn test_format_result_custom_formatter_error_fallback() {
        let formatter: OutputFormatter = Box::new(|_val| {
            Err("formatter exploded".into())
        });
        let router = ExecutionRouter {
            executor: None,
            validate_inputs: false,
            output_formatter: Some(formatter),
        };
        // Should fall back to JSON when formatter returns an error
        let result = router.format_result(&json!({"key": "value"}));
        assert_eq!(result, r#"{"key":"value"}"#);
    }

    // ==================================================================
    // Task 4: Context construction tests
    // ==================================================================

    fn make_send_notification() -> (SendNotificationFn, Arc<std::sync::Mutex<Vec<Value>>>) {
        let captured = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
        let captured_clone = captured.clone();
        let sn: SendNotificationFn = Arc::new(move |val| {
            let captured = captured_clone.clone();
            Box::pin(async move {
                captured.lock().unwrap().push(val);
                Ok(())
            })
        });
        (sn, captured)
    }

    struct MockSession {
        result: ElicitResult,
    }

    #[async_trait]
    impl SessionHandle for MockSession {
        async fn elicit_form(
            &self,
            _message: &str,
            _requested_schema: &Value,
        ) -> Result<ElicitResult, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.result.clone())
        }
    }

    struct FailingSession;

    #[async_trait]
    impl SessionHandle for FailingSession {
        async fn elicit_form(
            &self,
            _message: &str,
            _requested_schema: &Value,
        ) -> Result<ElicitResult, Box<dyn std::error::Error + Send + Sync>> {
            Err("session error".into())
        }
    }

    #[test]
    fn test_build_context_with_progress_callback() {
        let (sn, _) = make_send_notification();
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok-1".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        assert!(data.contains_key(MCP_PROGRESS_KEY));
    }

    #[test]
    fn test_build_context_without_progress() {
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: None,
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        assert!(!data.contains_key(MCP_PROGRESS_KEY));
    }

    #[test]
    fn test_build_context_with_elicit_callback() {
        let session = Arc::new(MockSession {
            result: ElicitResult {
                action: crate::helpers::ElicitAction::Accept,
                content: None,
            },
        });
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: Some(session),
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        assert!(data.contains_key(MCP_ELICIT_KEY));
    }

    #[test]
    fn test_build_context_without_session() {
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: None,
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        assert!(!data.contains_key(MCP_ELICIT_KEY));
    }

    #[test]
    fn test_build_context_with_identity() {
        let identity = json!({"id": "user-1", "type": "user"});
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: None,
            identity: Some(identity.clone()),
        };
        let (ctx, _) = ExecutionRouter::build_context(&extra);
        assert_eq!(ctx["identity"], identity);
    }

    #[test]
    fn test_build_context_without_identity() {
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: None,
            identity: None,
        };
        let (ctx, _) = ExecutionRouter::build_context(&extra);
        assert!(ctx.get("identity").is_none());
    }

    #[tokio::test]
    async fn test_progress_callback_sends_notification() {
        let (sn, captured) = make_send_notification();
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok-1".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        let cb = data
            .get(MCP_PROGRESS_KEY)
            .unwrap()
            .downcast_ref::<crate::helpers::ProgressCallback>()
            .unwrap();
        cb(0.5, Some(1.0), Some("halfway".into())).await;

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        let notif = &notifications[0];
        assert_eq!(notif["method"], "notifications/progress");
        assert_eq!(notif["params"]["progressToken"], "tok-1");
        assert_eq!(notif["params"]["message"], "halfway");
    }

    #[tokio::test]
    async fn test_progress_callback_includes_message() {
        let (sn, captured) = make_send_notification();
        let extra = CallExtra {
            progress_token: Some(ProgressToken::Integer(42)),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        let cb = data
            .get(MCP_PROGRESS_KEY)
            .unwrap()
            .downcast_ref::<crate::helpers::ProgressCallback>()
            .unwrap();
        cb(1.0, None, Some("doing stuff".into())).await;

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications[0]["params"]["message"], "doing stuff");
    }

    #[tokio::test]
    async fn test_progress_callback_omits_message() {
        let (sn, captured) = make_send_notification();
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        let cb = data
            .get(MCP_PROGRESS_KEY)
            .unwrap()
            .downcast_ref::<crate::helpers::ProgressCallback>()
            .unwrap();
        cb(1.0, None, None).await;

        let notifications = captured.lock().unwrap();
        assert!(notifications[0]["params"].get("message").is_none());
    }

    #[tokio::test]
    async fn test_elicit_callback_returns_result() {
        use crate::helpers::ElicitAction;
        let session = Arc::new(MockSession {
            result: ElicitResult {
                action: ElicitAction::Accept,
                content: Some(json!({"name": "Alice"})),
            },
        });
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: Some(session),
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        let cb = data
            .get(MCP_ELICIT_KEY)
            .unwrap()
            .downcast_ref::<crate::helpers::ElicitCallback>()
            .unwrap();
        let result = cb("confirm?".into(), None).await.unwrap();
        assert_eq!(result.action, ElicitAction::Accept);
        assert_eq!(result.content.unwrap()["name"], "Alice");
    }

    // ==================================================================
    // Task 5: Non-streaming path tests
    // ==================================================================

    /// A mock executor that always fails.
    struct FailingExecutor {
        error: ExecutorError,
    }

    impl FailingExecutor {
        fn new(code: &str, message: &str) -> Self {
            Self {
                error: ExecutorError::Execution {
                    code: code.to_string(),
                    message: message.to_string(),
                    details: None,
                },
            }
        }

        fn with_guidance(code: &str, message: &str, details: Value) -> Self {
            Self {
                error: ExecutorError::Execution {
                    code: code.to_string(),
                    message: message.to_string(),
                    details: Some(details),
                },
            }
        }
    }

    #[async_trait]
    impl Executor for FailingExecutor {
        async fn call_async(
            &self,
            _module_id: &str,
            _inputs: &Value,
            _context: Option<&Value>,
        ) -> Result<Value, ExecutorError> {
            // Clone the error fields manually since ExecutorError doesn't impl Clone
            match &self.error {
                ExecutorError::Execution {
                    code,
                    message,
                    details,
                } => Err(ExecutorError::Execution {
                    code: code.clone(),
                    message: message.clone(),
                    details: details.clone(),
                }),
                _ => Err(ExecutorError::Other("unknown".into())),
            }
        }
    }

    #[tokio::test]
    async fn test_call_async_success() {
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, None);
        let (content, is_error, _trace_id) = router
            .handle_call_async("test.module", &json!({"key": "value"}), None)
            .await;
        assert!(!is_error);
        assert_eq!(content.len(), 1);
        assert_eq!(content[0].content_type, "text");
        // Result should be JSON-formatted
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["module"], "test.module");
        assert_eq!(parsed["echo"]["key"], "value");
    }

    #[tokio::test]
    async fn test_call_async_success_with_trace_id() {
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, None);
        let ctx = json!({"trace_id": "trace-abc-123"});
        let (_content, is_error, trace_id) = router
            .handle_call_async("test.module", &json!({}), Some(&ctx))
            .await;
        assert!(!is_error);
        assert_eq!(trace_id, Some("trace-abc-123".to_string()));
    }

    #[tokio::test]
    async fn test_call_async_success_custom_formatter() {
        let formatter: OutputFormatter = Box::new(|val| {
            let obj = val.as_object().unwrap();
            Ok(format!("keys={}", obj.len()))
        });
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, Some(formatter));
        let (content, is_error, _) = router
            .handle_call_async("test.module", &json!({"a": 1}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        assert_eq!(text, "keys=2"); // "module" and "echo"
    }

    #[tokio::test]
    async fn test_call_async_error_mapped() {
        let router = ExecutionRouter::new(
            Box::new(FailingExecutor::new("MODULE_EXECUTE_ERROR", "division by zero")),
            false,
            None,
        );
        let (content, is_error, _) = router
            .handle_call_async("test.module", &json!({}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("division by zero"));
    }

    #[tokio::test]
    async fn test_call_async_error_text_includes_message() {
        let router = ExecutionRouter::new(
            Box::new(FailingExecutor::new("ERR", "something went wrong")),
            false,
            None,
        );
        let (content, is_error, _) = router
            .handle_call_async("mod", &json!({}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("something went wrong"));
    }

    #[tokio::test]
    async fn test_call_async_error_is_error_true() {
        let router = ExecutionRouter::new(
            Box::new(FailingExecutor::new("ERR", "fail")),
            false,
            None,
        );
        let (_, is_error, _) = router
            .handle_call_async("mod", &json!({}), None)
            .await;
        assert!(is_error);
    }

    #[tokio::test]
    async fn test_call_async_error_no_trace_id() {
        let router = ExecutionRouter::new(
            Box::new(FailingExecutor::new("ERR", "fail")),
            false,
            None,
        );
        let (_, _, trace_id) = router
            .handle_call_async("mod", &json!({}), None)
            .await;
        assert!(trace_id.is_none());
    }

    #[tokio::test]
    async fn test_call_async_error_with_ai_guidance() {
        let details = json!({
            "retryable": true,
            "aiGuidance": "Try with smaller input",
        });
        let router = ExecutionRouter::new(
            Box::new(FailingExecutor::with_guidance("ERR", "too large", details)),
            false,
            None,
        );
        let (content, is_error, _) = router
            .handle_call_async("mod", &json!({}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("too large"));
        assert!(text.contains("retryable"));
        assert!(text.contains("Try with smaller input"));
    }

    #[test]
    fn test_build_error_text_simple() {
        let error = ExecutorError::Execution {
            code: "ERR".into(),
            message: "Something broke".into(),
            details: None,
        };
        let text = ExecutionRouter::build_error_text(&error);
        assert_eq!(text, "Something broke");
    }

    #[test]
    fn test_build_error_text_with_retryable() {
        let error = ExecutorError::Execution {
            code: "ERR".into(),
            message: "Temporary failure".into(),
            details: Some(json!({"retryable": true})),
        };
        let text = ExecutionRouter::build_error_text(&error);
        assert!(text.starts_with("Temporary failure"));
        assert!(text.contains(r#""retryable":true"#));
    }

    #[test]
    fn test_build_error_text_with_all_guidance() {
        let error = ExecutorError::Execution {
            code: "ERR".into(),
            message: "Failed".into(),
            details: Some(json!({
                "retryable": true,
                "aiGuidance": "reduce input",
                "userFixable": false,
                "suggestion": "try again"
            })),
        };
        let text = ExecutionRouter::build_error_text(&error);
        assert!(text.starts_with("Failed\n\n"));
        assert!(text.contains("retryable"));
        assert!(text.contains("aiGuidance"));
        assert!(text.contains("userFixable"));
        assert!(text.contains("suggestion"));
    }

    // ==================================================================
    // Task 6: Streaming path tests
    // ==================================================================

    /// Mock executor that streams specific chunks.
    struct StreamingMockExecutor {
        chunks: Vec<Result<Value, ExecutorError>>,
    }

    #[async_trait]
    impl Executor for StreamingMockExecutor {
        async fn call_async(
            &self,
            _module_id: &str,
            _inputs: &Value,
            _context: Option<&Value>,
        ) -> Result<Value, ExecutorError> {
            Ok(json!({"fallback": true}))
        }

        fn stream(
            &self,
            _module_id: &str,
            _inputs: &Value,
            _context: Option<&Value>,
        ) -> Option<Pin<Box<dyn Stream<Item = Result<Value, ExecutorError>> + Send>>> {
            // We need to clone the data for the stream
            let chunks: Vec<Result<Value, ExecutorError>> = self
                .chunks
                .iter()
                .map(|r| match r {
                    Ok(v) => Ok(v.clone()),
                    Err(ExecutorError::Execution {
                        code,
                        message,
                        details,
                    }) => Err(ExecutorError::Execution {
                        code: code.clone(),
                        message: message.clone(),
                        details: details.clone(),
                    }),
                    _ => Err(ExecutorError::Other("clone error".into())),
                })
                .collect();
            Some(Box::pin(tokio_stream::iter(chunks)))
        }
    }

    /// Mock executor that returns None for stream (no streaming support).
    struct NonStreamingExecutor;

    #[async_trait]
    impl Executor for NonStreamingExecutor {
        async fn call_async(
            &self,
            _module_id: &str,
            _inputs: &Value,
            _context: Option<&Value>,
        ) -> Result<Value, ExecutorError> {
            Ok(json!({"non_streaming": true}))
        }
        // stream() defaults to None
    }

    #[tokio::test]
    async fn test_stream_single_chunk() {
        let (sn, captured) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![Ok(json!({"result": 42}))],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("tok-1".into());
        let (content, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["result"], 42);

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 1);
    }

    #[tokio::test]
    async fn test_stream_multiple_chunks_merged() {
        let (sn, captured) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![
                Ok(json!({"a": 1})),
                Ok(json!({"b": 2})),
                Ok(json!({"c": 3})),
            ],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("tok".into());
        let (content, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["b"], 2);
        assert_eq!(parsed["c"], 3);

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 3);
    }

    #[tokio::test]
    async fn test_stream_empty() {
        let (sn, captured) = make_send_notification();
        let executor = StreamingMockExecutor { chunks: vec![] };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("tok".into());
        let (content, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed, json!({}));

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 0);
    }

    #[tokio::test]
    async fn test_stream_progress_notification_structure() {
        let (sn, captured) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![Ok(json!({"key": "val"}))],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("my-token".into());
        router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;

        let notifications = captured.lock().unwrap();
        let notif = &notifications[0];
        assert_eq!(notif["method"], "notifications/progress");
        assert_eq!(notif["params"]["progressToken"], "my-token");
        assert_eq!(notif["params"]["progress"], 1); // 1-indexed
        assert!(notif["params"]["total"].is_null());
        // message is the JSON serialized chunk
        let msg = notif["params"]["message"].as_str().unwrap();
        let chunk: Value = serde_json::from_str(msg).unwrap();
        assert_eq!(chunk["key"], "val");
    }

    #[tokio::test]
    async fn test_stream_progress_token_string() {
        let (sn, captured) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![Ok(json!({}))],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("str-tok".into());
        router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications[0]["params"]["progressToken"], "str-tok");
    }

    #[tokio::test]
    async fn test_stream_progress_token_integer() {
        let (sn, captured) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![Ok(json!({}))],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::Integer(99);
        router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications[0]["params"]["progressToken"], 99);
    }

    #[tokio::test]
    async fn test_stream_accumulates_nested() {
        let (sn, _) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![
                Ok(json!({"data": {"x": 1}})),
                Ok(json!({"data": {"y": 2}})),
            ],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("tok".into());
        let (content, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["data"]["x"], 1);
        assert_eq!(parsed["data"]["y"], 2);
    }

    #[tokio::test]
    async fn test_stream_error_mid_stream() {
        let (sn, _) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![
                Ok(json!({"a": 1})),
                Err(ExecutorError::Execution {
                    code: "ERR".into(),
                    message: "stream broke".into(),
                    details: None,
                }),
                Ok(json!({"c": 3})),
            ],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("tok".into());
        let (content, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("stream broke"));
    }

    #[tokio::test]
    async fn test_stream_error_is_error_true() {
        let (sn, _) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![Err(ExecutorError::Execution {
                code: "ERR".into(),
                message: "fail".into(),
                details: None,
            })],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("tok".into());
        let (_, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(is_error);
    }

    #[tokio::test]
    async fn test_stream_result_formatted() {
        let (sn, _) = make_send_notification();
        let formatter: OutputFormatter = Box::new(|val| {
            let obj = val.as_object().unwrap();
            Ok(format!("formatted:{}", obj.len()))
        });
        let executor = StreamingMockExecutor {
            chunks: vec![Ok(json!({"a": 1})), Ok(json!({"b": 2}))],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, Some(formatter));
        let token = ProgressToken::String("tok".into());
        let (content, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        assert_eq!(text, "formatted:2");
    }

    #[tokio::test]
    async fn test_stream_result_has_trace_id() {
        let (sn, _) = make_send_notification();
        let executor = StreamingMockExecutor {
            chunks: vec![Ok(json!({"a": 1}))],
        };
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let token = ProgressToken::String("tok".into());
        let ctx = json!({"trace_id": "trace-xyz"});
        let (_, is_error, trace_id) = router
            .handle_stream("mod", &json!({}), &token, &sn, Some(&ctx))
            .await;
        assert!(!is_error);
        assert_eq!(trace_id, Some("trace-xyz".to_string()));
    }

    #[tokio::test]
    async fn test_stream_fallback_to_non_streaming() {
        let (sn, captured) = make_send_notification();
        let router = ExecutionRouter::new(Box::new(NonStreamingExecutor), false, None);
        let token = ProgressToken::String("tok".into());
        let (content, is_error, _) = router
            .handle_stream("mod", &json!({}), &token, &sn, None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["non_streaming"], true);

        // No progress notifications should be sent (fell back)
        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 0);
    }

    // ==================================================================
    // Task 7: Input validation tests
    // ==================================================================

    #[test]
    fn test_format_validation_errors_single_field() {
        let errors = vec![ValidationError {
            field: Some("name".to_string()),
            message: "is required".to_string(),
            errors: vec![],
        }];
        let result = ExecutionRouter::format_validation_errors(&errors);
        assert_eq!(result, "name: is required");
    }

    #[test]
    fn test_format_validation_errors_multiple_fields() {
        let errors = vec![
            ValidationError {
                field: Some("name".to_string()),
                message: "is required".to_string(),
                errors: vec![],
            },
            ValidationError {
                field: Some("age".to_string()),
                message: "must be positive".to_string(),
                errors: vec![],
            },
        ];
        let result = ExecutionRouter::format_validation_errors(&errors);
        assert_eq!(result, "name: is required; age: must be positive");
    }

    #[test]
    fn test_format_validation_errors_nested() {
        let errors = vec![ValidationError {
            field: Some("address".to_string()),
            message: "has errors".to_string(),
            errors: vec![
                ValidationError {
                    field: Some("street".to_string()),
                    message: "is required".to_string(),
                    errors: vec![],
                },
                ValidationError {
                    field: Some("zip".to_string()),
                    message: "is invalid".to_string(),
                    errors: vec![],
                },
            ],
        }];
        let result = ExecutionRouter::format_validation_errors(&errors);
        assert_eq!(result, "street: is required; zip: is invalid");
    }

    #[test]
    fn test_format_validation_errors_no_field() {
        let errors = vec![ValidationError {
            field: None,
            message: "general error".to_string(),
            errors: vec![],
        }];
        let result = ExecutionRouter::format_validation_errors(&errors);
        assert_eq!(result, "general error");
    }

    #[test]
    fn test_format_validation_errors_nested_no_field() {
        let errors = vec![ValidationError {
            field: None,
            message: "parent".to_string(),
            errors: vec![ValidationError {
                field: None,
                message: "child error".to_string(),
                errors: vec![],
            }],
        }];
        let result = ExecutionRouter::format_validation_errors(&errors);
        assert_eq!(result, "?: child error");
    }

    #[tokio::test]
    async fn test_validation_disabled_skips() {
        // With validate_inputs=false, executor.validate() is never relevant
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), false, None);
        // FullMockExecutor would return invalid for {"invalid": true}
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({"invalid": true}), None)
            .await;
        // Should NOT return validation error — validation disabled
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(!text.contains("Validation failed"));
    }

    #[tokio::test]
    async fn test_validation_enabled_valid_inputs() {
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), true, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({"name": "ok"}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(!text.contains("Validation failed"));
    }

    #[tokio::test]
    async fn test_validation_enabled_invalid_inputs() {
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), true, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({"invalid": true}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("Validation failed"));
        assert!(text.contains("invalid: field is not allowed"));
    }

    #[tokio::test]
    async fn test_validation_returns_is_error_true() {
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), true, None);
        let (_, is_error, trace_id) = router
            .handle_call_with_extra("mod", &json!({"invalid": true}), None)
            .await;
        assert!(is_error);
        assert!(trace_id.is_none());
    }

    #[tokio::test]
    async fn test_validation_executor_lacks_validate() {
        // MockExecutor returns None from validate() — validation should be skipped
        let router = ExecutionRouter::new(Box::new(MockExecutor), true, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("test.module", &json!({"anything": true}), None)
            .await;
        // Should proceed to execution since validate returns None
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(!text.contains("Validation failed"));
    }

    // ==================================================================
    // Task 8: handle_call orchestrator tests
    // ==================================================================

    #[test]
    fn test_new_default_formatter() {
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, None);
        assert!(router.output_formatter.is_none());
        assert!(!router.validate_inputs);
        assert!(router.executor.is_some());
    }

    #[test]
    fn test_new_custom_formatter() {
        let formatter: OutputFormatter = Box::new(|_| Ok("custom".into()));
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, Some(formatter));
        assert!(router.output_formatter.is_some());
    }

    #[test]
    fn test_new_validate_inputs_flag() {
        let router = ExecutionRouter::new(Box::new(MockExecutor), true, None);
        assert!(router.validate_inputs);
    }

    #[tokio::test]
    async fn test_handle_call_non_streaming() {
        // Without progress_token, routes to non-streaming path
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("test.module", &json!({"key": "val"}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["module"], "test.module");
    }

    #[tokio::test]
    async fn test_handle_call_streaming() {
        // With progress_token + send_notification + executor.stream(), routes to streaming
        let (sn, captured) = make_send_notification();
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), false, None);
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), Some(extra))
            .await;
        assert!(!is_error);
        // FullMockExecutor streams {"a":1}, {"b":2}
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["b"], 2);

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_call_streaming_missing_send_notification() {
        // With progress_token but no send_notification, falls back to non-streaming
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), false, None);
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok".into())),
            send_notification: None,
            session: None,
            identity: None,
        };
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({"x": 1}), Some(extra))
            .await;
        assert!(!is_error);
        // Should fall back to call_async which returns inputs as-is for FullMockExecutor
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["x"], 1);
    }

    #[tokio::test]
    async fn test_handle_call_streaming_executor_no_stream() {
        // Executor doesn't support streaming, falls back to non-streaming
        let (sn, captured) = make_send_notification();
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, None);
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (content, is_error, _) = router
            .handle_call_with_extra("test.module", &json!({}), Some(extra))
            .await;
        assert!(!is_error);
        // MockExecutor has no stream(), should fall back
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["module"], "test.module");

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_call_validation_before_execution() {
        // With validate_inputs=true, validation runs before execution
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), true, None);
        let (_, is_error, _) = router
            .handle_call_with_extra("mod", &json!({"name": "ok"}), None)
            .await;
        assert!(!is_error);
    }

    #[tokio::test]
    async fn test_handle_call_validation_failure_short_circuits() {
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), true, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({"invalid": true}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("Validation failed"));
    }

    #[tokio::test]
    async fn test_handle_call_passes_identity() {
        // Identity from CallExtra is passed to context
        let identity = json!({"id": "user-1"});
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: None,
            identity: Some(identity),
        };
        // Use IdentityCapturingExecutor to verify
        let router = ExecutionRouter::new(Box::new(IdentityCapturingExecutor), false, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), Some(extra))
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["identity"]["id"], "user-1");
    }

    #[tokio::test]
    async fn test_handle_call_no_extra() {
        // None extra works correctly (non-streaming, no callbacks)
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("test.module", &json!({"k": "v"}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("test.module"));
    }

    /// Executor that captures the context identity.
    struct IdentityCapturingExecutor;

    #[async_trait]
    impl Executor for IdentityCapturingExecutor {
        async fn call_async(
            &self,
            _module_id: &str,
            _inputs: &Value,
            context: Option<&Value>,
        ) -> Result<Value, ExecutorError> {
            // Return the context so test can inspect it
            Ok(context.cloned().unwrap_or(Value::Null))
        }
    }

    // ==================================================================
    // Task 8: handle_call (Value-based) orchestrator tests
    // ==================================================================

    #[tokio::test]
    async fn test_handle_call_value_non_streaming() {
        let router = ExecutionRouter::new(Box::new(MockExecutor), false, None);
        let (content, is_error, _) = router
            .handle_call("test.module", &json!({"key": "val"}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["module"], "test.module");
    }

    #[tokio::test]
    async fn test_handle_call_value_with_identity() {
        let router = ExecutionRouter::new(Box::new(IdentityCapturingExecutor), false, None);
        let extra = json!({"identity": {"id": "user-2"}});
        let (content, is_error, _) = router
            .handle_call("mod", &json!({}), Some(&extra))
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["identity"]["id"], "user-2");
    }

    #[tokio::test]
    async fn test_handle_call_value_validation_failure() {
        let router = ExecutionRouter::new(Box::new(FullMockExecutor), true, None);
        let (content, is_error, _) = router
            .handle_call("mod", &json!({"invalid": true}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("Validation failed"));
    }

    // ==================================================================
    // Task 9: Integration tests (end-to-end)
    // ==================================================================

    /// Configurable mock executor for integration tests.
    struct ConfigurableExecutor {
        call_result: std::sync::Mutex<Option<Result<Value, ExecutorError>>>,
        stream_chunks: std::sync::Mutex<Option<Vec<Result<Value, ExecutorError>>>>,
        validate_result: std::sync::Mutex<Option<ValidationResult>>,
        calls: std::sync::Mutex<Vec<(String, Value)>>,
    }

    impl ConfigurableExecutor {
        fn new() -> Self {
            Self {
                call_result: std::sync::Mutex::new(None),
                stream_chunks: std::sync::Mutex::new(None),
                validate_result: std::sync::Mutex::new(None),
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn with_call_result(self, result: Result<Value, ExecutorError>) -> Self {
            *self.call_result.lock().unwrap() = Some(result);
            self
        }

        fn with_stream_chunks(self, chunks: Vec<Result<Value, ExecutorError>>) -> Self {
            *self.stream_chunks.lock().unwrap() = Some(chunks);
            self
        }

        fn with_validate_result(self, result: ValidationResult) -> Self {
            *self.validate_result.lock().unwrap() = Some(result);
            self
        }

        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl Executor for ConfigurableExecutor {
        async fn call_async(
            &self,
            module_id: &str,
            inputs: &Value,
            _context: Option<&Value>,
        ) -> Result<Value, ExecutorError> {
            self.calls
                .lock()
                .unwrap()
                .push((module_id.to_string(), inputs.clone()));
            match self.call_result.lock().unwrap().take() {
                Some(result) => match result {
                    Ok(v) => Ok(v),
                    Err(ExecutorError::Execution {
                        code,
                        message,
                        details,
                    }) => Err(ExecutorError::Execution {
                        code,
                        message,
                        details,
                    }),
                    Err(ExecutorError::Validation(msg)) => Err(ExecutorError::Validation(msg)),
                    Err(e) => Err(e),
                },
                None => Ok(inputs.clone()),
            }
        }

        fn stream(
            &self,
            _module_id: &str,
            _inputs: &Value,
            _context: Option<&Value>,
        ) -> Option<Pin<Box<dyn Stream<Item = Result<Value, ExecutorError>> + Send>>> {
            let chunks = self.stream_chunks.lock().unwrap().take()?;
            let cloned: Vec<Result<Value, ExecutorError>> = chunks
                .into_iter()
                .map(|r| match r {
                    Ok(v) => Ok(v),
                    Err(ExecutorError::Execution {
                        code,
                        message,
                        details,
                    }) => Err(ExecutorError::Execution {
                        code,
                        message,
                        details,
                    }),
                    Err(ExecutorError::Validation(msg)) => Err(ExecutorError::Validation(msg)),
                    Err(e) => Err(e),
                })
                .collect();
            Some(Box::pin(tokio_stream::iter(cloned)))
        }

        fn validate(
            &self,
            _module_id: &str,
            _inputs: &Value,
            _context: Option<&Value>,
        ) -> Option<ValidationResult> {
            self.validate_result.lock().unwrap().take()
        }
    }

    #[tokio::test]
    async fn test_e2e_simple_call() {
        let executor = ConfigurableExecutor::new()
            .with_call_result(Ok(json!({"status": "ok", "count": 42})));
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let (content, is_error, trace_id) = router
            .handle_call_with_extra("my.tool", &json!({"input": 1}), None)
            .await;
        assert!(!is_error);
        assert!(trace_id.is_some()); // context always gets a trace_id
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["count"], 42);
    }

    #[tokio::test]
    async fn test_e2e_call_with_custom_formatter() {
        let executor = ConfigurableExecutor::new()
            .with_call_result(Ok(json!({"x": 1, "y": 2})));
        let formatter: OutputFormatter = Box::new(|val| {
            let keys: Vec<&str> = val.as_object().unwrap().keys().map(|k| k.as_str()).collect();
            Ok(format!("Keys: {}", keys.join(", ")))
        });
        let router = ExecutionRouter::new(Box::new(executor), false, Some(formatter));
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.starts_with("Keys: "));
    }

    #[tokio::test]
    async fn test_e2e_call_error_mapped() {
        let executor = ConfigurableExecutor::new().with_call_result(Err(
            ExecutorError::Execution {
                code: "ERR_DIVIDE".into(),
                message: "cannot divide by zero".into(),
                details: None,
            },
        ));
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("cannot divide by zero"));
    }

    #[tokio::test]
    async fn test_e2e_call_with_identity() {
        let executor = ConfigurableExecutor::new();
        let router = ExecutionRouter::new(Box::new(IdentityCapturingExecutor), false, None);
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: None,
            identity: Some(json!({"user": "alice", "role": "admin"})),
        };
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), Some(extra))
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["identity"]["user"], "alice");
        assert_eq!(parsed["identity"]["role"], "admin");
    }

    #[tokio::test]
    async fn test_e2e_streaming_three_chunks() {
        let (sn, captured) = make_send_notification();
        let executor = ConfigurableExecutor::new().with_stream_chunks(vec![
            Ok(json!({"part": "a"})),
            Ok(json!({"data": 1})),
            Ok(json!({"part": "c", "data": 2})),
        ]);
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok-e2e".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), Some(extra))
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["part"], "c"); // last "part" wins
        assert_eq!(parsed["data"], 2); // last "data" wins

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 3);
    }

    #[tokio::test]
    async fn test_e2e_streaming_notification_content() {
        let (sn, captured) = make_send_notification();
        let executor = ConfigurableExecutor::new()
            .with_stream_chunks(vec![Ok(json!({"step": "done"}))]);
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok-notif".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        router
            .handle_call_with_extra("mod", &json!({}), Some(extra))
            .await;

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        let notif = &notifications[0];
        assert_eq!(notif["method"], "notifications/progress");
        assert_eq!(notif["params"]["progressToken"], "tok-notif");
        assert_eq!(notif["params"]["progress"], 1);
        let msg = notif["params"]["message"].as_str().unwrap();
        let chunk: Value = serde_json::from_str(msg).unwrap();
        assert_eq!(chunk["step"], "done");
    }

    #[tokio::test]
    async fn test_e2e_streaming_error_mid_stream() {
        let (sn, _) = make_send_notification();
        let executor = ConfigurableExecutor::new().with_stream_chunks(vec![
            Ok(json!({"a": 1})),
            Err(ExecutorError::Execution {
                code: "STREAM_ERR".into(),
                message: "stream failed at chunk 2".into(),
                details: None,
            }),
        ]);
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), Some(extra))
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("stream failed at chunk 2"));
    }

    #[tokio::test]
    async fn test_e2e_streaming_fallback_no_support() {
        // Executor without stream support falls back to non-streaming
        let (sn, captured) = make_send_notification();
        let executor = ConfigurableExecutor::new()
            .with_call_result(Ok(json!({"fallback": true})));
        // Don't set stream_chunks — stream() returns None
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), Some(extra))
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["fallback"], true);

        let notifications = captured.lock().unwrap();
        assert_eq!(notifications.len(), 0);
    }

    #[tokio::test]
    async fn test_e2e_validation_pass() {
        let executor = ConfigurableExecutor::new()
            .with_validate_result(ValidationResult {
                valid: true,
                errors: vec![],
            })
            .with_call_result(Ok(json!({"result": "success"})));
        let router = ExecutionRouter::new(Box::new(executor), true, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["result"], "success");
    }

    #[tokio::test]
    async fn test_e2e_validation_fail() {
        let executor = ConfigurableExecutor::new()
            .with_validate_result(ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    field: Some("email".to_string()),
                    message: "is not a valid email".to_string(),
                    errors: vec![],
                }],
            })
            .with_call_result(Ok(json!({"should": "not reach"})));
        let router = ExecutionRouter::new(Box::new(executor), true, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("Validation failed"));
        assert!(text.contains("email: is not a valid email"));
    }

    #[tokio::test]
    async fn test_e2e_validation_disabled() {
        // Even with invalid validation result, execution proceeds when disabled
        let executor = ConfigurableExecutor::new()
            .with_validate_result(ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    field: Some("x".to_string()),
                    message: "bad".to_string(),
                    errors: vec![],
                }],
            })
            .with_call_result(Ok(json!({"executed": true})));
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["executed"], true);
    }

    #[tokio::test]
    async fn test_e2e_no_extra() {
        let executor = ConfigurableExecutor::new()
            .with_call_result(Ok(json!({"ok": true})));
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), None)
            .await;
        assert!(!is_error);
        let text = content[0].data.as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["ok"], true);
    }

    #[tokio::test]
    async fn test_e2e_error_with_ai_guidance() {
        let executor = ConfigurableExecutor::new().with_call_result(Err(
            ExecutorError::Execution {
                code: "RATE_LIMIT".into(),
                message: "Too many requests".into(),
                details: Some(json!({
                    "retryable": true,
                    "aiGuidance": "Wait 10 seconds and retry",
                    "suggestion": "reduce request rate"
                })),
            },
        ));
        let router = ExecutionRouter::new(Box::new(executor), false, None);
        let (content, is_error, _) = router
            .handle_call_with_extra("mod", &json!({}), None)
            .await;
        assert!(is_error);
        let text = content[0].data.as_str().unwrap();
        assert!(text.contains("Too many requests"));
        assert!(text.contains("retryable"));
        assert!(text.contains("Wait 10 seconds and retry"));
        assert!(text.contains("suggestion"));
    }

    #[tokio::test]
    async fn test_e2e_progress_callback_in_context() {
        // Verify that progress callback is accessible via context data
        let (sn, _) = make_send_notification();
        let extra = CallExtra {
            progress_token: Some(ProgressToken::String("tok".into())),
            send_notification: Some(sn),
            session: None,
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        assert!(data.contains_key(MCP_PROGRESS_KEY));
    }

    #[tokio::test]
    async fn test_e2e_elicit_callback_in_context() {
        let session = Arc::new(MockSession {
            result: ElicitResult {
                action: crate::helpers::ElicitAction::Accept,
                content: None,
            },
        });
        let extra = CallExtra {
            progress_token: None,
            send_notification: None,
            session: Some(session),
            identity: None,
        };
        let (_, data) = ExecutionRouter::build_context(&extra);
        assert!(data.contains_key(MCP_ELICIT_KEY));
    }
}
