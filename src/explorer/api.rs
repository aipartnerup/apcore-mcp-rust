//! API handlers for the explorer — JSON endpoints for listing and calling tools.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

use crate::auth::middleware::AUTH_IDENTITY;
use crate::auth::Authenticator;
use crate::explorer::mount::{CallResult, HandleCallFn, ToolInfo};

// ---------------------------------------------------------------------------
// ExplorerState
// ---------------------------------------------------------------------------

/// Shared state for explorer API handlers.
#[derive(Clone)]
pub struct ExplorerState {
    /// The registered tools.
    pub tools: Arc<Vec<ToolInfo>>,
    /// Optional callback for executing tool calls.
    pub handle_call: Option<HandleCallFn>,
    /// Whether tool execution is permitted.
    pub allow_execute: bool,
    /// Optional authenticator for protecting call endpoints.
    pub authenticator: Option<Arc<dyn Authenticator>>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// JSON response returned by the `call_tool` handler.
#[derive(Debug, Serialize)]
pub struct CallResponse {
    /// Content blocks produced by the tool.
    pub content: Vec<serde_json::Value>,
    /// Whether the tool execution resulted in an error.
    pub is_error: bool,
    /// Optional error code string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

/// JSON error body used for 4xx responses.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    detail: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /tools` — return the list of registered tools as JSON.
pub async fn list_tools(State(state): State<ExplorerState>) -> impl IntoResponse {
    Json(state.tools.as_ref().clone())
}

/// `POST /tools/{name}/call` — execute a tool and return the result.
pub async fn call_tool(
    State(state): State<ExplorerState>,
    Path(tool_name): Path<String>,
    headers: HeaderMap,
    Json(args): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 1. Check allow_execute
    if !state.allow_execute {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::to_value(ErrorResponse {
                error: "Forbidden".to_string(),
                detail: "Tool execution is disabled".to_string(),
            })
            .unwrap()),
        )
            .into_response();
    }

    // 2. Validate tool exists
    if !state.tools.iter().any(|t| t.name == tool_name) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::to_value(ErrorResponse {
                error: "Not Found".to_string(),
                detail: format!("Tool '{}' not found", tool_name),
            })
            .unwrap()),
        )
            .into_response();
    }

    // 3. Auth check (if authenticator is set)
    let identity = if let Some(ref authenticator) = state.authenticator {
        let header_map: HashMap<String, String> = headers
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|v| (name.to_string(), v.to_string()))
            })
            .collect();
        let id = authenticator.authenticate(&header_map).await;
        if id.is_none() {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::to_value(ErrorResponse {
                    error: "Unauthorized".to_string(),
                    detail: "Missing or invalid Bearer token".to_string(),
                })
                .unwrap()),
            )
                .into_response();
        }
        id
    } else {
        None
    };

    // 4. Execute the tool call
    let handle_call = match state.handle_call {
        Some(ref f) => f.clone(),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(ErrorResponse {
                    error: "Internal Error".to_string(),
                    detail: "No call handler configured".to_string(),
                })
                .unwrap()),
            )
                .into_response();
        }
    };

    let (content, is_error, error_code): CallResult = if identity.is_some() {
        AUTH_IDENTITY
            .scope(identity, async { handle_call(tool_name, args).await })
            .await
    } else {
        handle_call(tool_name, args).await
    };

    let resp = CallResponse {
        content,
        is_error,
        error_code,
    };

    (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explorer::mount::ToolInfo;
    use axum::body::Body;
    use axum::http::Request;
    use axum::Router;
    use axum::routing::{get, post};
    use serde_json::json;
    use tower::ServiceExt;

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

    fn make_router(state: ExplorerState) -> Router {
        Router::new()
            .route("/tools", get(list_tools))
            .route("/tools/{name}/call", post(call_tool))
            .with_state(state)
    }

    #[tokio::test]
    async fn list_tools_returns_all_tools() {
        let state = ExplorerState {
            tools: Arc::new(sample_tools()),
            handle_call: None,
            allow_execute: false,
            authenticator: None,
        };
        let app = make_router(state);

        let req = Request::get("/tools").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let tools: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["name"], "tool_one");
        assert_eq!(tools[1]["name"], "tool_two");
        // Verify camelCase serialization
        assert!(tools[0].get("inputSchema").is_some());
    }

    #[tokio::test]
    async fn call_tool_returns_403_when_execute_disabled() {
        let state = ExplorerState {
            tools: Arc::new(sample_tools()),
            handle_call: Some(mock_handle_call()),
            allow_execute: false,
            authenticator: None,
        };
        let app = make_router(state);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Forbidden");
    }

    #[tokio::test]
    async fn call_tool_returns_404_for_unknown_tool() {
        let state = ExplorerState {
            tools: Arc::new(sample_tools()),
            handle_call: Some(mock_handle_call()),
            allow_execute: true,
            authenticator: None,
        };
        let app = make_router(state);

        let req = Request::post("/tools/nonexistent/call")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Not Found");
    }

    #[tokio::test]
    async fn call_tool_success() {
        let state = ExplorerState {
            tools: Arc::new(sample_tools()),
            handle_call: Some(mock_handle_call()),
            allow_execute: true,
            authenticator: None,
        };
        let app = make_router(state);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"arg":"value"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(!json["is_error"].as_bool().unwrap());
        assert!(json["content"][0]["text"].as_str().unwrap().contains("tool_one"));
        // error_code should be absent (skip_serializing_if)
        assert!(json.get("error_code").is_none());
    }

    #[tokio::test]
    async fn call_tool_returns_401_when_auth_fails() {
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

        let state = ExplorerState {
            tools: Arc::new(sample_tools()),
            handle_call: Some(mock_handle_call()),
            allow_execute: true,
            authenticator: Some(Arc::new(RejectAuth)),
        };
        let app = make_router(state);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn call_tool_with_auth_bridges_identity() {
        use async_trait::async_trait;
        use std::collections::HashMap;
        use crate::auth::protocol::Identity;

        #[derive(Clone)]
        struct AcceptAuth;

        #[async_trait]
        impl Authenticator for AcceptAuth {
            async fn authenticate(&self, headers: &HashMap<String, String>) -> Option<Identity> {
                let auth = headers.get("authorization")?;
                if auth.starts_with("Bearer ") {
                    Some(Identity {
                        id: "test-user".to_string(),
                        identity_type: "human".to_string(),
                        roles: vec!["user".to_string()],
                        attrs: Default::default(),
                    })
                } else {
                    None
                }
            }
        }

        // handle_call that checks AUTH_IDENTITY
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

        let state = ExplorerState {
            tools: Arc::new(sample_tools()),
            handle_call: Some(handle_call),
            allow_execute: true,
            authenticator: Some(Arc::new(AcceptAuth)),
        };
        let app = make_router(state);

        let req = Request::post("/tools/tool_one/call")
            .header("content-type", "application/json")
            .header("authorization", "Bearer test-token")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["content"][0]["text"], "test-user");
    }
}
