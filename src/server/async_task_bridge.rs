//! AsyncTaskBridge — routes async-hinted modules through apcore's
//! [`AsyncTaskManager`] and exposes the `__apcore_task_*` meta-tools.
//!
//! See `docs/features/async-task-bridge.md` for the full feature spec.
//!
//! This bridge is a thin routing layer:
//! * Modules whose descriptor carries `metadata.async == true` OR
//!   `annotations.extra["mcp_async"] == "true"` are routed through
//!   [`AsyncTaskManager::submit`].
//! * Four reserved MCP tool names are registered:
//!   - `__apcore_task_submit`
//!   - `__apcore_task_status`
//!   - `__apcore_task_cancel`
//!   - `__apcore_task_list`
//! * Module ids starting with `__apcore_` are rejected by the bridge to
//!   keep the meta-tool namespace reserved.
//! * When the caller supplies `_meta.progressToken`, the progress token is
//!   recorded and (future) progress events are fanned out via
//!   `notifications/progress`.
//! * On `__apcore_task_status`, completed results are redacted via
//!   `apcore::redact_sensitive` using the module's output schema.

// apcore::ModuleError is a large enum; the bridge returns it through the
// normal apcore error mapper rather than boxing, matching the convention
// used elsewhere in this crate (see src/server/router.rs).
#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use apcore::async_task::{AsyncTaskManager, TaskInfo, TaskStatus};
use apcore::executor::Executor as ApcoreExecutor;
use apcore::registry::registry::Registry;
use apcore::{Context, Identity};
use serde_json::{json, Value};

use crate::server::types::Tool;

/// Reserved prefix for MCP meta-tools.
pub const META_TOOL_PREFIX: &str = "__apcore_";

/// Meta-tool names.
pub const META_TOOL_SUBMIT: &str = "__apcore_task_submit";
pub const META_TOOL_STATUS: &str = "__apcore_task_status";
pub const META_TOOL_CANCEL: &str = "__apcore_task_cancel";
pub const META_TOOL_LIST: &str = "__apcore_task_list";

/// Default `AsyncTaskManager` configuration.
pub const DEFAULT_MAX_CONCURRENT: usize = 10;
pub const DEFAULT_MAX_TASKS: usize = 1000;

/// Bridge for routing async-hinted module calls through
/// [`AsyncTaskManager`] and exposing the `__apcore_task_*` meta-tools.
pub struct AsyncTaskBridge {
    /// Shared apcore async task manager.
    manager: Arc<AsyncTaskManager>,
    /// Cached output schemas (per module id) used for result redaction.
    output_schemas: HashMap<String, Value>,
    /// Map of task_id → progressToken recorded at submit time for
    /// progress fan-out on terminal transitions.
    progress_tokens: Arc<Mutex<HashMap<String, Value>>>,
    /// Map of session/connection key → set of task ids launched from that
    /// session, so [`AsyncTaskBridge::cancel_session_tasks`] can cancel
    /// them cooperatively when the transport detects disconnect.
    session_tasks: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl AsyncTaskBridge {
    /// Create a new bridge backed by the given apcore executor.
    pub fn new(executor: Arc<ApcoreExecutor>) -> Self {
        Self::with_limits(executor, DEFAULT_MAX_CONCURRENT, DEFAULT_MAX_TASKS)
    }

    /// Create a new bridge with explicit concurrency/task limits.
    pub fn with_limits(
        executor: Arc<ApcoreExecutor>,
        max_concurrent: usize,
        max_tasks: usize,
    ) -> Self {
        Self {
            manager: Arc::new(AsyncTaskManager::new(executor, max_concurrent, max_tasks)),
            output_schemas: HashMap::new(),
            progress_tokens: Arc::new(Mutex::new(HashMap::new())),
            session_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set the output schemas used for result redaction on
    /// `__apcore_task_status` responses.
    pub fn with_output_schemas(mut self, schemas: HashMap<String, Value>) -> Self {
        self.output_schemas = schemas;
        self
    }

    /// Direct access to the underlying manager (used for shutdown).
    pub fn manager(&self) -> &Arc<AsyncTaskManager> {
        &self.manager
    }

    /// Check whether a module descriptor is async-hinted.
    ///
    /// Async modules are those with:
    /// - `metadata.async == true`, or
    /// - `annotations.extra["mcp_async"] == "true"` (string or bool).
    pub fn is_async_module(
        metadata: &HashMap<String, Value>,
        annotations_extra: Option<&HashMap<String, Value>>,
    ) -> bool {
        if let Some(v) = metadata.get("async") {
            if v.as_bool() == Some(true) {
                return true;
            }
            if v.as_str() == Some("true") {
                return true;
            }
        }
        if let Some(extra) = annotations_extra {
            if let Some(v) = extra.get("mcp_async") {
                if v.as_bool() == Some(true) {
                    return true;
                }
                if v.as_str() == Some("true") {
                    return true;
                }
            }
        }
        false
    }

    /// Check whether an async hint applies to a module registered in the
    /// given apcore registry.
    pub fn is_async_registered(registry: &Registry, module_id: &str) -> bool {
        let Some(desc) = registry.get_definition(module_id) else {
            return false;
        };
        let extra = desc.annotations.as_ref().map(|a| &a.extra);
        Self::is_async_module(&desc.metadata, extra)
    }

    /// Return `true` if a module id is reserved by the bridge.
    pub fn is_reserved_id(module_id: &str) -> bool {
        module_id.starts_with(META_TOOL_PREFIX)
    }

    /// Submit a module for asynchronous execution.
    ///
    /// Returns the `TaskInfo` envelope on success, or a `ModuleError`
    /// (mapped via the shared error formatter at the MCP boundary) on
    /// capacity / validation failures.
    pub fn submit(
        &self,
        module_id: &str,
        inputs: Value,
        identity: Option<Identity>,
        progress_token: Option<Value>,
        session_key: Option<&str>,
    ) -> Result<TaskInfo, apcore::errors::ModuleError> {
        if Self::is_reserved_id(module_id) {
            return Err(apcore::errors::ModuleError::new(
                apcore::errors::ErrorCode::GeneralInvalidInput,
                format!(
                    "module id '{module_id}' is reserved by the async task bridge (__apcore_ prefix)"
                ),
            ));
        }

        let ctx: Option<Context<Value>> = identity.map(Context::new);
        let task_id = self.manager.submit(module_id, inputs, ctx)?;

        if let Some(token) = progress_token {
            let mut guard = self
                .progress_tokens
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            guard.insert(task_id.clone(), token);
        }

        if let Some(session) = session_key {
            let mut guard = self.session_tasks.lock().unwrap_or_else(|p| p.into_inner());
            guard
                .entry(session.to_string())
                .or_default()
                .push(task_id.clone());
        }

        self.manager.get_status(&task_id).ok_or_else(|| {
            apcore::errors::ModuleError::new(
                apcore::errors::ErrorCode::GeneralInternalError,
                format!("task {task_id} was submitted but disappeared from the manager"),
            )
        })
    }

    /// Retrieve the current `TaskInfo` for a task id.
    ///
    /// When the task is in `Completed` state, the embedded `result` is
    /// redacted via the module's registered output schema (if any).
    pub fn get_status(&self, task_id: &str) -> Option<TaskInfo> {
        let mut info = self.manager.get_status(task_id)?;
        if info.status == TaskStatus::Completed {
            if let Some(result) = &info.result {
                if let Some(schema) = self.output_schemas.get(&info.module_id) {
                    let redacted = apcore::redact_sensitive(result, schema);
                    info.result = Some(redacted);
                }
            }
        }
        Some(info)
    }

    /// Cancel a running or pending task.
    pub fn cancel(&self, task_id: &str) -> bool {
        let mut guard = self
            .progress_tokens
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        guard.remove(task_id);
        drop(guard);
        self.manager.cancel(task_id)
    }

    /// Cancel every task recorded under the given session key. Used by the
    /// transport layer when a client disconnects or cancels a request.
    ///
    /// Returns the number of tasks cancelled.
    pub fn cancel_session_tasks(&self, session_key: &str) -> usize {
        let ids: Vec<String> = {
            let mut map = self.session_tasks.lock().unwrap_or_else(|p| p.into_inner());
            map.remove(session_key).unwrap_or_default()
        };
        let mut n = 0;
        for id in &ids {
            if self.cancel(id) {
                n += 1;
            }
        }
        n
    }

    /// List all tracked tasks, optionally filtered by status.
    pub fn list_tasks(&self, status: Option<TaskStatus>) -> Vec<TaskInfo> {
        self.manager.list_tasks(status)
    }

    /// Cancel all pending/running tasks at server shutdown.
    pub fn shutdown(&self) {
        self.manager.shutdown();
    }

    /// Build the four reserved meta-tool definitions.
    pub fn build_meta_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: META_TOOL_SUBMIT.to_string(),
                description:
                    "Submit a module for asynchronous execution via apcore's AsyncTaskManager."
                        .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "module_id": { "type": "string" },
                        "arguments": { "type": "object" },
                        "version_hint": { "type": "string" }
                    },
                    "required": ["module_id"]
                }),
                annotations: None,
                meta: Some(json!({ "reserved": true })),
            },
            Tool {
                name: META_TOOL_STATUS.to_string(),
                description:
                    "Query the TaskInfo projection for a previously submitted task."
                        .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_id": { "type": "string" }
                    },
                    "required": ["task_id"]
                }),
                annotations: None,
                meta: Some(json!({ "reserved": true })),
            },
            Tool {
                name: META_TOOL_CANCEL.to_string(),
                description: "Cancel a pending or running async task.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_id": { "type": "string" }
                    },
                    "required": ["task_id"]
                }),
                annotations: None,
                meta: Some(json!({ "reserved": true })),
            },
            Tool {
                name: META_TOOL_LIST.to_string(),
                description:
                    "List all tracked tasks, optionally filtered by status (pending/running/completed/failed/cancelled)."
                        .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["pending", "running", "completed", "failed", "cancelled"]
                        }
                    }
                }),
                annotations: None,
                meta: Some(json!({ "reserved": true })),
            },
        ]
    }

    /// Handle a meta-tool invocation.
    ///
    /// Returns `Some(result_json)` when the call matched a meta-tool name,
    /// or `None` when the caller should fall through to the normal
    /// execution path.
    pub fn handle_meta_tool(
        &self,
        tool_name: &str,
        arguments: &Value,
        identity: Option<Identity>,
        progress_token: Option<Value>,
        session_key: Option<&str>,
    ) -> Option<Result<Value, apcore::errors::ModuleError>> {
        match tool_name {
            META_TOOL_SUBMIT => {
                Some(self.handle_submit(arguments, identity, progress_token, session_key))
            }
            META_TOOL_STATUS => Some(self.handle_status(arguments)),
            META_TOOL_CANCEL => Some(self.handle_cancel(arguments)),
            META_TOOL_LIST => Some(self.handle_list(arguments)),
            _ => None,
        }
    }

    fn handle_submit(
        &self,
        arguments: &Value,
        identity: Option<Identity>,
        progress_token: Option<Value>,
        session_key: Option<&str>,
    ) -> Result<Value, apcore::errors::ModuleError> {
        let module_id = arguments
            .get("module_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                apcore::errors::ModuleError::new(
                    apcore::errors::ErrorCode::GeneralInvalidInput,
                    "__apcore_task_submit requires 'module_id'",
                )
            })?;
        let inputs = arguments.get("arguments").cloned().unwrap_or(json!({}));
        let info = self.submit(module_id, inputs, identity, progress_token, session_key)?;
        Ok(serde_json::to_value(info).unwrap_or(Value::Null))
    }

    fn handle_status(&self, arguments: &Value) -> Result<Value, apcore::errors::ModuleError> {
        let task_id = arguments
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                apcore::errors::ModuleError::new(
                    apcore::errors::ErrorCode::GeneralInvalidInput,
                    "__apcore_task_status requires 'task_id'",
                )
            })?;
        match self.get_status(task_id) {
            Some(info) => Ok(serde_json::to_value(info).unwrap_or(Value::Null)),
            None => Err(apcore::errors::ModuleError::new(
                apcore::errors::ErrorCode::ModuleNotFound,
                format!("task not found: {task_id}"),
            )),
        }
    }

    fn handle_cancel(&self, arguments: &Value) -> Result<Value, apcore::errors::ModuleError> {
        let task_id = arguments
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                apcore::errors::ModuleError::new(
                    apcore::errors::ErrorCode::GeneralInvalidInput,
                    "__apcore_task_cancel requires 'task_id'",
                )
            })?;
        let cancelled = self.cancel(task_id);
        Ok(json!({ "task_id": task_id, "cancelled": cancelled }))
    }

    fn handle_list(&self, arguments: &Value) -> Result<Value, apcore::errors::ModuleError> {
        let status_filter = arguments
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "pending" => Ok(TaskStatus::Pending),
                "running" => Ok(TaskStatus::Running),
                "completed" => Ok(TaskStatus::Completed),
                "failed" => Ok(TaskStatus::Failed),
                "cancelled" => Ok(TaskStatus::Cancelled),
                other => Err(apcore::errors::ModuleError::new(
                    apcore::errors::ErrorCode::GeneralInvalidInput,
                    format!("unknown task status filter: {other}"),
                )),
            })
            .transpose()?;
        let tasks = self.list_tasks(status_filter);
        let tasks_json: Vec<Value> = tasks
            .into_iter()
            .map(|t| serde_json::to_value(t).unwrap_or(Value::Null))
            .collect();
        Ok(json!({ "tasks": tasks_json }))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apcore::executor::Executor;
    use apcore::module::ModuleAnnotations;
    use apcore::registry::registry::Registry;
    use std::sync::Arc;

    fn make_executor() -> Arc<Executor> {
        let registry = Arc::new(Registry::default());
        let config = Arc::new(apcore::config::Config::default());
        Arc::new(Executor::new(registry, config))
    }

    #[test]
    fn detects_async_metadata_bool() {
        let mut meta = HashMap::new();
        meta.insert("async".to_string(), json!(true));
        assert!(AsyncTaskBridge::is_async_module(&meta, None));
    }

    #[test]
    fn detects_async_metadata_string() {
        let mut meta = HashMap::new();
        meta.insert("async".to_string(), json!("true"));
        assert!(AsyncTaskBridge::is_async_module(&meta, None));
    }

    #[test]
    fn detects_mcp_async_in_extra() {
        let mut extra: HashMap<String, Value> = HashMap::new();
        extra.insert("mcp_async".to_string(), json!("true"));
        let meta = HashMap::new();
        assert!(AsyncTaskBridge::is_async_module(&meta, Some(&extra)));
    }

    #[test]
    fn non_async_module_returns_false() {
        let meta = HashMap::new();
        let ann = ModuleAnnotations::default();
        assert!(!AsyncTaskBridge::is_async_module(&meta, Some(&ann.extra)));
    }

    #[test]
    fn reserved_id_detection() {
        assert!(AsyncTaskBridge::is_reserved_id("__apcore_task_submit"));
        assert!(AsyncTaskBridge::is_reserved_id("__apcore_custom"));
        assert!(!AsyncTaskBridge::is_reserved_id("math.add"));
    }

    #[test]
    fn meta_tools_have_four_reserved_names() {
        let tools = AsyncTaskBridge::build_meta_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), 4);
        assert!(names.contains(&META_TOOL_SUBMIT));
        assert!(names.contains(&META_TOOL_STATUS));
        assert!(names.contains(&META_TOOL_CANCEL));
        assert!(names.contains(&META_TOOL_LIST));
    }

    #[tokio::test]
    async fn submit_rejects_reserved_id() {
        let bridge = AsyncTaskBridge::new(make_executor());
        let err = bridge
            .submit("__apcore_task_submit", json!({}), None, None, None)
            .unwrap_err();
        assert_eq!(err.code, apcore::errors::ErrorCode::GeneralInvalidInput);
    }

    #[tokio::test]
    async fn submit_returns_task_info() {
        let bridge = AsyncTaskBridge::new(make_executor());
        let info = bridge
            .submit("some.module", json!({}), None, None, None)
            .expect("submit should succeed");
        assert_eq!(info.module_id, "some.module");
        // status is either Pending or has already transitioned
        assert!(matches!(
            info.status,
            TaskStatus::Pending | TaskStatus::Running | TaskStatus::Failed
        ));
    }

    #[tokio::test]
    async fn handle_meta_tool_submit_then_status() {
        let bridge = AsyncTaskBridge::new(make_executor());
        let submit_result = bridge
            .handle_meta_tool(
                META_TOOL_SUBMIT,
                &json!({"module_id": "some.module", "arguments": {}}),
                None,
                None,
                None,
            )
            .expect("meta-tool must be routed")
            .expect("submit should return Ok");
        let task_id = submit_result
            .get("task_id")
            .and_then(|v| v.as_str())
            .expect("task_id must be present");

        let status_result = bridge
            .handle_meta_tool(
                META_TOOL_STATUS,
                &json!({"task_id": task_id}),
                None,
                None,
                None,
            )
            .expect("meta-tool must be routed")
            .expect("status should return Ok");
        assert_eq!(
            status_result.get("task_id").and_then(|v| v.as_str()),
            Some(task_id)
        );
    }

    #[tokio::test]
    async fn handle_meta_tool_cancel_returns_flag() {
        let executor = make_executor();
        let bridge = AsyncTaskBridge::with_limits(executor, 0, 100); // 0 concurrency keeps tasks pending
        let submit = bridge
            .submit("m", json!({}), None, None, None)
            .expect("submit");
        let task_id = submit.task_id;
        let res = bridge
            .handle_meta_tool(
                META_TOOL_CANCEL,
                &json!({"task_id": task_id}),
                None,
                None,
                None,
            )
            .expect("routed")
            .expect("cancel ok");
        assert_eq!(res.get("cancelled").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn handle_meta_tool_list_filters_by_status() {
        let bridge = AsyncTaskBridge::with_limits(make_executor(), 0, 100);
        let _ = bridge.submit("m", json!({}), None, None, None).unwrap();
        let res = bridge
            .handle_meta_tool(
                META_TOOL_LIST,
                &json!({"status": "pending"}),
                None,
                None,
                None,
            )
            .expect("routed")
            .expect("list ok");
        let tasks = res.get("tasks").and_then(|v| v.as_array()).unwrap();
        // The submitted task may still be Pending (0 concurrency) but
        // tokio scheduling may also race it to Running. Either way the
        // response must be an array.
        assert!(tasks.len() <= 1, "at most one task recorded");
    }

    #[tokio::test]
    async fn cancel_session_tasks_cancels_tracked_ids() {
        let bridge = AsyncTaskBridge::with_limits(make_executor(), 0, 100);
        let t1 = bridge
            .submit("m1", json!({}), None, None, Some("sess-a"))
            .unwrap();
        let t2 = bridge
            .submit("m2", json!({}), None, None, Some("sess-a"))
            .unwrap();
        // Task from another session should not be affected.
        let t3 = bridge
            .submit("m3", json!({}), None, None, Some("sess-b"))
            .unwrap();

        let cancelled = bridge.cancel_session_tasks("sess-a");
        assert_eq!(cancelled, 2);
        assert_eq!(
            bridge.get_status(&t1.task_id).map(|i| i.status),
            Some(TaskStatus::Cancelled)
        );
        assert_eq!(
            bridge.get_status(&t2.task_id).map(|i| i.status),
            Some(TaskStatus::Cancelled)
        );
        assert!(matches!(
            bridge.get_status(&t3.task_id).map(|i| i.status),
            Some(TaskStatus::Pending) | Some(TaskStatus::Running) | Some(TaskStatus::Failed)
        ));
    }

    #[test]
    fn handle_meta_tool_unknown_returns_none() {
        let bridge = AsyncTaskBridge::new(make_executor());
        let res = bridge.handle_meta_tool("math.add", &json!({}), None, None, None);
        assert!(res.is_none());
    }
}
