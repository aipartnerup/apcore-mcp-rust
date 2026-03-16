//! Explorer mount — creates an axum router for browsing and introspecting
//! the registered MCP tools.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::http::header;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::auth::Authenticator;
use crate::explorer::api::{self, ExplorerState};

// ---------------------------------------------------------------------------
// ToolInfo
// ---------------------------------------------------------------------------

/// Metadata about a single MCP tool, suitable for display in the explorer UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolInfo {
    /// The tool's unique name.
    pub name: String,
    /// A human-readable description of what the tool does.
    pub description: String,
    /// The JSON Schema describing the tool's input parameters.
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

// ---------------------------------------------------------------------------
// HandleCallFn
// ---------------------------------------------------------------------------

/// The result of executing a tool call: a list of content blocks, an
/// `is_error` flag, and an optional error message.
pub type CallResult = (Vec<serde_json::Value>, bool, Option<String>);

/// Async callback used to execute a tool by name with the given arguments.
pub type HandleCallFn = Arc<
    dyn Fn(String, serde_json::Value) -> Pin<Box<dyn Future<Output = CallResult> + Send>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// ExplorerConfig
// ---------------------------------------------------------------------------

/// Configuration for the explorer UI and API routes.
pub struct ExplorerConfig {
    /// The tools to expose in the explorer.
    pub tools: Vec<ToolInfo>,
    /// Optional callback for executing tool calls from the UI.
    pub handle_call: Option<HandleCallFn>,
    /// Whether the explorer UI allows executing tools.
    pub allow_execute: bool,
    /// URL prefix where the explorer is mounted (e.g. `"/explorer"`).
    pub explorer_prefix: String,
    /// Optional authenticator for protecting explorer endpoints.
    pub authenticator: Option<Arc<dyn Authenticator>>,
    /// Page title shown in the browser tab and heading.
    pub title: String,
    /// Optional project name shown in the explorer footer.
    pub project_name: Option<String>,
    /// Optional project URL linked in the explorer footer.
    pub project_url: Option<String>,
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
            handle_call: None,
            allow_execute: false,
            explorer_prefix: "/explorer".to_string(),
            authenticator: None,
            title: "MCP Tool Explorer".to_string(),
            project_name: None,
            project_url: None,
        }
    }
}

impl ExplorerConfig {
    /// Create a new `ExplorerConfig` with the given tools and defaults for
    /// everything else.
    pub fn new(tools: Vec<ToolInfo>) -> Self {
        Self {
            tools,
            ..Default::default()
        }
    }

    /// Set the `allow_execute` flag.
    pub fn allow_execute(mut self, allow: bool) -> Self {
        self.allow_execute = allow;
        self
    }

    /// Set the explorer URL prefix.
    pub fn explorer_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.explorer_prefix = prefix.into();
        self
    }

    /// Set the page title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the project name shown in the footer.
    pub fn project_name(mut self, name: impl Into<String>) -> Self {
        self.project_name = Some(name.into());
        self
    }

    /// Set the project URL linked in the footer.
    pub fn project_url(mut self, url: impl Into<String>) -> Self {
        self.project_url = Some(url.into());
        self
    }

    /// Set the authenticator.
    pub fn authenticator(mut self, auth: Arc<dyn Authenticator>) -> Self {
        self.authenticator = Some(auth);
        self
    }

    /// Set the tool-execution callback.
    pub fn handle_call(mut self, f: HandleCallFn) -> Self {
        self.handle_call = Some(f);
        self
    }
}

/// Create an axum [`Router`] that serves the explorer UI and API.
///
/// The explorer provides:
/// - A list of all registered tools with their schemas
/// - A test interface for invoking tools interactively
/// - JSON API for programmatic access
pub fn create_explorer_mount(config: ExplorerConfig) -> Router {
    use crate::explorer::templates::render_html;

    let html = render_html(
        &config.title,
        config.project_name.as_deref(),
        config.project_url.as_deref(),
        config.allow_execute,
    );

    let state = ExplorerState {
        tools: Arc::new(config.tools),
        handle_call: config.handle_call,
        allow_execute: config.allow_execute,
        authenticator: config.authenticator,
    };

    Router::new()
        .route(
            "/",
            get(move || async move {
                ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
            }),
        )
        .route("/tools", get(api::list_tools))
        .route("/tools/{name}/call", post(api::call_tool))
        .with_state(state)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::json;
    use tower::ServiceExt;

    // -- ExplorerConfig defaults -------------------------------------------

    #[test]
    fn config_default_values() {
        let cfg = ExplorerConfig::default();
        assert_eq!(cfg.explorer_prefix, "/explorer");
        assert!(!cfg.allow_execute);
        assert_eq!(cfg.title, "MCP Tool Explorer");
        assert!(cfg.authenticator.is_none());
        assert!(cfg.handle_call.is_none());
        assert!(cfg.tools.is_empty());
        assert!(cfg.project_name.is_none());
        assert!(cfg.project_url.is_none());
    }

    #[test]
    fn config_builder_overrides() {
        let cfg = ExplorerConfig::new(vec![])
            .allow_execute(true)
            .explorer_prefix("/tools-ui")
            .title("My Tools")
            .project_name("demo")
            .project_url("https://example.com");

        assert!(cfg.allow_execute);
        assert_eq!(cfg.explorer_prefix, "/tools-ui");
        assert_eq!(cfg.title, "My Tools");
        assert_eq!(cfg.project_name.as_deref(), Some("demo"));
        assert_eq!(cfg.project_url.as_deref(), Some("https://example.com"));
    }

    // -- ToolInfo serialization --------------------------------------------

    #[test]
    fn tool_info_serializes_with_correct_field_names() {
        let tool = ToolInfo {
            name: "add".to_string(),
            description: "Add two numbers".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                }
            }),
        };

        let serialized = serde_json::to_value(&tool).unwrap();
        assert_eq!(serialized["name"], "add");
        assert_eq!(serialized["description"], "Add two numbers");
        // Must use camelCase "inputSchema", not snake_case
        assert!(serialized.get("inputSchema").is_some());
        assert!(serialized.get("input_schema").is_none());
    }

    #[test]
    fn tool_info_round_trip() {
        let tool = ToolInfo {
            name: "search".to_string(),
            description: "Search documents".to_string(),
            input_schema: json!({"type": "object"}),
        };

        let json_str = serde_json::to_string(&tool).unwrap();
        let deserialized: ToolInfo = serde_json::from_str(&json_str).unwrap();
        assert_eq!(tool, deserialized);
    }

    // -- HandleCallFn type check ------------------------------------------

    #[test]
    fn handle_call_fn_is_send_sync() {
        // Compile-time assertion: HandleCallFn must be Send + Sync.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HandleCallFn>();
    }

    // -- Helpers for integration tests ------------------------------------

    fn sample_tools() -> Vec<ToolInfo> {
        vec![
            ToolInfo {
                name: "tool_one".to_string(),
                description: "First tool".to_string(),
                input_schema: json!({"type": "object"}),
            },
            ToolInfo {
                name: "tool_two".to_string(),
                description: "Second tool".to_string(),
                input_schema: json!({"type": "object"}),
            },
        ]
    }

    fn mock_handle_call() -> HandleCallFn {
        Arc::new(|name: String, args: serde_json::Value| {
            Box::pin(async move {
                let content = vec![json!({"type": "text", "text": format!("called {} with {}", name, args)})];
                (content, false, None)
            })
        })
    }

    // -- Integration: full lifecycle --------------------------------------

    #[tokio::test]
    async fn integration_html_page_served() {
        let config = ExplorerConfig::new(sample_tools())
            .allow_execute(true)
            .handle_call(mock_handle_call())
            .title("Test Explorer");
        let app = create_explorer_mount(config);

        let req = Request::get("/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("<title>Test Explorer</title>"));
        assert!(html.contains(r#"data-allow-execute="true""#));
    }

    #[tokio::test]
    async fn integration_tools_endpoint() {
        let config = ExplorerConfig::new(sample_tools())
            .allow_execute(true)
            .handle_call(mock_handle_call());
        let app = create_explorer_mount(config);

        let req = Request::get("/tools").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let tools: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tools.len(), 2);
    }

    #[tokio::test]
    async fn integration_call_tool_success() {
        let config = ExplorerConfig::new(sample_tools())
            .allow_execute(true)
            .handle_call(mock_handle_call());
        let app = create_explorer_mount(config);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"arg": "value"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(!json["is_error"].as_bool().unwrap());
        assert!(json["content"][0]["text"].as_str().unwrap().contains("tool_one"));
    }

    #[tokio::test]
    async fn integration_call_nonexistent_tool_returns_404() {
        let config = ExplorerConfig::new(sample_tools())
            .allow_execute(true)
            .handle_call(mock_handle_call());
        let app = create_explorer_mount(config);

        let req = Request::post("/tools/nonexistent/call")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 404);
    }

    // -- Integration: execution disabled ----------------------------------

    #[tokio::test]
    async fn integration_execute_disabled_returns_403() {
        let config = ExplorerConfig::new(sample_tools())
            .allow_execute(false)
            .handle_call(mock_handle_call());
        let app = create_explorer_mount(config);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 403);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Forbidden");
    }

    #[tokio::test]
    async fn integration_execute_disabled_listing_still_works() {
        let config = ExplorerConfig::new(sample_tools()).allow_execute(false);
        let app = create_explorer_mount(config);

        let req = Request::get("/tools").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    // -- Integration: auth required ---------------------------------------

    #[tokio::test]
    async fn integration_auth_required_no_token_returns_401() {
        use async_trait::async_trait;
        use std::collections::HashMap;
        use crate::auth::protocol::Identity;

        struct RejectAuth;

        #[async_trait]
        impl Authenticator for RejectAuth {
            async fn authenticate(&self, _headers: &HashMap<String, String>) -> Option<Identity> {
                None
            }
        }

        let config = ExplorerConfig::new(sample_tools())
            .allow_execute(true)
            .handle_call(mock_handle_call())
            .authenticator(Arc::new(RejectAuth));
        let app = create_explorer_mount(config);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 401);
    }

    #[tokio::test]
    async fn integration_auth_with_valid_token_succeeds() {
        use async_trait::async_trait;
        use std::collections::HashMap;
        use crate::auth::protocol::Identity;
        use crate::auth::middleware::AUTH_IDENTITY;

        #[derive(Clone)]
        struct AcceptAuth;

        #[async_trait]
        impl Authenticator for AcceptAuth {
            async fn authenticate(&self, headers: &HashMap<String, String>) -> Option<Identity> {
                let auth = headers.get("authorization")?;
                if auth.starts_with("Bearer ") {
                    Some(Identity {
                        id: "authed-user".to_string(),
                        identity_type: "human".to_string(),
                        roles: vec!["user".to_string()],
                        attrs: Default::default(),
                    })
                } else {
                    None
                }
            }
        }

        // handle_call that verifies AUTH_IDENTITY is set
        let handle_call: HandleCallFn = Arc::new(|_name, _args| {
            Box::pin(async move {
                let identity_id = AUTH_IDENTITY
                    .try_with(|id| {
                        id.as_ref().map(|i| i.id.clone()).unwrap_or_else(|| "none".to_string())
                    })
                    .unwrap_or_else(|_| "no-task-local".to_string());
                let content = vec![json!({"type": "text", "text": identity_id})];
                (content, false, None)
            })
        });

        let config = ExplorerConfig::new(sample_tools())
            .allow_execute(true)
            .handle_call(handle_call)
            .authenticator(Arc::new(AcceptAuth));
        let app = create_explorer_mount(config);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .header("authorization", "Bearer valid")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // Verify AUTH_IDENTITY was bridged during execution
        assert_eq!(json["content"][0]["text"], "authed-user");
    }

    // -- HTML: allow_execute=false has no data attribute -------------------

    #[tokio::test]
    async fn integration_html_no_execute_attribute_when_disabled() {
        let config = ExplorerConfig::new(sample_tools()).allow_execute(false);
        let app = create_explorer_mount(config);

        let req = Request::get("/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(!html.contains(r#"data-allow-execute="true""#));
    }
}
