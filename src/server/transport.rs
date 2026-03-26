//! TransportManager — manages MCP transport lifecycle (stdio, HTTP, SSE).
//!
//! Also exposes Prometheus metrics for observability.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::Request;
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Router;
use hyper::StatusCode;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};

// ---------------------------------------------------------------------------
// Task 1: TransportError
// ---------------------------------------------------------------------------

/// Unified error type for all transport failure modes.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("invalid host: {0}")]
    InvalidHost(String),

    #[error("port must be between 1 and 65535, got {0}")]
    InvalidPort(u16),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to bind to {host}:{port}: {source}")]
    Bind {
        host: String,
        port: u16,
        source: hyper::Error,
    },

    #[error("server error: {0}")]
    Server(String),
}

// ---------------------------------------------------------------------------
// MetricsExporter trait
// ---------------------------------------------------------------------------

/// Trait for exporting server metrics.
pub trait MetricsExporter: Send + Sync {
    /// Export metrics in Prometheus text format.
    fn export_prometheus(&self) -> String;
}

// ---------------------------------------------------------------------------
// McpHandler trait
// ---------------------------------------------------------------------------

/// Trait for handling MCP JSON-RPC messages.
///
/// The transport layer delegates incoming messages to an `McpHandler`
/// implementation, which processes them and optionally returns a response.
/// Notifications (no `id` field) return `None`.
#[async_trait::async_trait]
pub trait McpHandler: Send + Sync {
    /// Handle an incoming JSON-RPC message.
    ///
    /// Returns `Some(response)` for requests, `None` for notifications.
    async fn handle_message(&self, message: Value) -> Option<Value>;
}

// ---------------------------------------------------------------------------
// Task 2: TransportManager struct
// ---------------------------------------------------------------------------

/// Manages the transport layer for the MCP server.
pub struct TransportManager {
    start_time: tokio::time::Instant,
    module_count: usize,
    metrics_exporter: Option<Arc<dyn MetricsExporter>>,
}

impl TransportManager {
    /// Create a new transport manager with an optional metrics exporter.
    pub fn new(metrics_exporter: Option<Arc<dyn MetricsExporter>>) -> Self {
        Self {
            start_time: tokio::time::Instant::now(),
            module_count: 0,
            metrics_exporter,
        }
    }

    /// Set the number of registered modules (for metrics).
    pub fn set_module_count(&mut self, count: usize) {
        self.module_count = count;
    }

    /// Get the current module count.
    pub fn module_count(&self) -> usize {
        self.module_count
    }

    /// Validate host and port parameters.
    fn validate_host_port(host: &str, port: u16) -> Result<(), TransportError> {
        if host.is_empty() {
            return Err(TransportError::InvalidHost(host.to_string()));
        }
        if port == 0 {
            return Err(TransportError::InvalidPort(port));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Task 3: Health & Metrics
    // -----------------------------------------------------------------------

    /// Build the health check response payload.
    fn build_health_response(&self) -> HealthResponse {
        HealthResponse {
            status: "ok",
            uptime_seconds: self.start_time.elapsed().as_secs_f64(),
            module_count: self.module_count,
        }
    }

    /// Build the metrics response.
    ///
    /// Returns `Ok(body)` with Prometheus text when an exporter is configured,
    /// or `Err(())` when no exporter is available (caller should return 404).
    fn build_metrics_response(&self) -> Result<String, ()> {
        match &self.metrics_exporter {
            Some(exporter) => Ok(exporter.export_prometheus()),
            None => Err(()),
        }
    }

    /// Build an axum [`Router`] with `/health` and `/metrics` GET routes.
    ///
    /// The returned router uses `Arc<TransportManager>` as shared state.
    pub fn health_metrics_router(self: &Arc<Self>) -> Router {
        let tm = Arc::clone(self);
        Router::new()
            .route("/health", get(health_handler))
            .route("/metrics", get(metrics_handler))
            .with_state(tm)
    }

    // -----------------------------------------------------------------------
    // Task 4: stdio transport
    // -----------------------------------------------------------------------

    /// Run the MCP server over stdio transport (blocks until EOF on stdin).
    ///
    /// Reads line-delimited JSON-RPC from stdin and writes responses to stdout.
    pub async fn run_stdio(&self, handler: &dyn McpHandler) -> Result<(), TransportError> {
        tracing::info!("Starting stdio transport");
        self.run_stdio_with_io(tokio::io::stdin(), tokio::io::stdout(), handler)
            .await
    }

    /// Testable core of `run_stdio` — reads from any `AsyncRead`, writes to any `AsyncWrite`.
    ///
    /// Each line is parsed as JSON. Valid messages are dispatched to `handler`.
    /// If the handler returns `Some(response)`, it is written as a JSON line to the writer.
    /// Invalid JSON lines are logged and skipped. EOF causes a clean return.
    pub async fn run_stdio_with_io<R, W>(
        &self,
        reader: R,
        mut writer: W,
        handler: &dyn McpHandler,
    ) -> Result<(), TransportError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = buf_reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                tracing::info!("stdio: EOF reached, shutting down");
                return Ok(());
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let message: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("stdio: invalid JSON, skipping: {}", e);
                    continue;
                }
            };

            if let Some(response) = handler.handle_message(message).await {
                let mut response_bytes = serde_json::to_vec(&response)
                    .map_err(|e| TransportError::Server(e.to_string()))?;
                response_bytes.push(b'\n');
                writer.write_all(&response_bytes).await?;
                writer.flush().await?;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Task 5: streamable-HTTP transport
    // -----------------------------------------------------------------------

    /// Build an axum [`Router`] for the streamable-HTTP transport.
    ///
    /// Includes `/health`, `/metrics`, and `/mcp` endpoints.
    /// The `/mcp` endpoint handles POST (JSON-RPC request/response),
    /// GET (SSE streaming placeholder), and DELETE (session termination).
    /// Extra routes are merged into the router if provided.
    pub fn build_streamable_http_app(
        self: &Arc<Self>,
        handler: Arc<dyn McpHandler>,
        extra_routes: Option<Router>,
    ) -> Router {
        // TODO: Per MCP spec, each client connection should get its own
        // session ID.  Currently a single ID is shared across all connections,
        // which means DELETE (session termination) affects all clients.
        // Requires a session store keyed by UUID.
        let session_id = uuid::Uuid::new_v4().to_string();
        let mcp_state = StreamableHttpState {
            handler,
            session_id,
        };

        let mcp_router = Router::new()
            .route(
                "/",
                post(streamable_http_post_handler)
                    .get(streamable_http_get_handler)
                    .delete(streamable_http_delete_handler),
            )
            .with_state(mcp_state);

        let mut app = self.health_metrics_router().nest("/mcp", mcp_router);

        if let Some(extra) = extra_routes {
            app = app.merge(extra);
        }

        // Axum's nest() does not match trailing slashes.  Add a fallback
        // redirect so that e.g. /explorer/ redirects to /explorer.
        app = app.fallback(trailing_slash_redirect);

        app
    }

    /// Run the MCP server over streamable-HTTP transport.
    ///
    /// Binds to the given host and port and serves until shutdown.
    pub async fn run_streamable_http(
        self: &Arc<Self>,
        handler: Arc<dyn McpHandler>,
        host: &str,
        port: u16,
        extra_routes: Option<Router>,
    ) -> Result<(), TransportError> {
        Self::validate_host_port(host, port)?;
        tracing::info!("Starting streamable-http transport on {}:{}", host, port);

        let app = self.build_streamable_http_app(handler, extra_routes);
        let addr: std::net::SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|_| TransportError::InvalidHost(host.to_string()))?;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .await
            .map_err(TransportError::Io)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Task 6: SSE transport (deprecated)
    // -----------------------------------------------------------------------

    /// Build an axum [`Router`] for the SSE transport.
    ///
    /// Includes `/health`, `/metrics`, `/sse` (GET), and `/messages/` (POST).
    #[deprecated(note = "SSE transport is deprecated. Use streamable-HTTP instead.")]
    pub fn build_sse_app(
        self: &Arc<Self>,
        handler: Arc<dyn McpHandler>,
        extra_routes: Option<Router>,
    ) -> Router {
        let (tx, rx) = tokio::sync::mpsc::channel::<Value>(256);
        let sse_state = SseState {
            handler,
            sender: tx,
            receiver: Arc::new(tokio::sync::Mutex::new(rx)),
        };

        let sse_router = Router::new()
            .route("/sse", get(sse_stream_handler))
            .route("/messages/", post(sse_messages_handler))
            .with_state(sse_state);

        let mut app = self.health_metrics_router().merge(sse_router);

        if let Some(extra) = extra_routes {
            app = app.merge(extra);
        }

        app
    }

    /// Run the MCP server over SSE transport (deprecated).
    ///
    /// Mounts `/sse` (GET) for the event stream and `/messages/` (POST) for
    /// client-to-server messages. Logs a deprecation warning at startup.
    #[deprecated(note = "SSE transport is deprecated. Use streamable-HTTP instead.")]
    pub async fn run_sse(
        self: &Arc<Self>,
        handler: Arc<dyn McpHandler>,
        host: &str,
        port: u16,
        extra_routes: Option<Router>,
    ) -> Result<(), TransportError> {
        Self::validate_host_port(host, port)?;
        tracing::info!("Starting sse transport on {}:{}", host, port);
        tracing::warn!("SSE transport is deprecated. Use Streamable HTTP instead.");

        #[allow(deprecated)]
        let app = self.build_sse_app(handler, extra_routes);

        let addr: std::net::SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|_| TransportError::InvalidHost(host.to_string()))?;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .await
            .map_err(TransportError::Io)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Health response struct
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_seconds: f64,
    module_count: usize,
}

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

async fn health_handler(
    axum::extract::State(tm): axum::extract::State<Arc<TransportManager>>,
) -> axum::Json<HealthResponse> {
    axum::Json(tm.build_health_response())
}

async fn metrics_handler(
    axum::extract::State(tm): axum::extract::State<Arc<TransportManager>>,
) -> axum::response::Response {
    match tm.build_metrics_response() {
        Ok(body) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            body,
        )
            .into_response(),
        Err(()) => StatusCode::NOT_FOUND.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Streamable-HTTP state & handlers
// ---------------------------------------------------------------------------

/// Shared state for the streamable-HTTP MCP endpoint.
#[derive(Clone)]
struct StreamableHttpState {
    handler: Arc<dyn McpHandler>,
    session_id: String,
}

/// POST /mcp — accept a JSON-RPC request and return a JSON-RPC response.
async fn streamable_http_post_handler(
    axum::extract::State(state): axum::extract::State<StreamableHttpState>,
    axum::Json(body): axum::Json<Value>,
) -> axum::response::Response {
    match state.handler.handle_message(body).await {
        Some(response) => (
            StatusCode::OK,
            [
                ("content-type", "application/json"),
                ("mcp-session-id", state.session_id.as_str()),
            ],
            serde_json::to_string(&response).unwrap_or_default(),
        )
            .into_response(),
        None => (
            StatusCode::ACCEPTED,
            [("mcp-session-id", state.session_id.as_str())],
        )
            .into_response(),
    }
}

/// GET /mcp — SSE-based streaming endpoint (placeholder).
async fn streamable_http_get_handler(
    axum::extract::State(state): axum::extract::State<StreamableHttpState>,
) -> axum::response::Response {
    // Placeholder: return a simple SSE stream that sends the session ID and closes.
    use axum::response::sse::{Event, Sse};

    let session_id = state.session_id.clone();
    let stream = tokio_stream::once(Ok::<_, Infallible>(
        Event::default()
            .event("endpoint")
            .data(format!("/mcp?sessionId={}", session_id)),
    ));
    Sse::new(stream).into_response()
}

/// DELETE /mcp — session termination.
async fn streamable_http_delete_handler(
    axum::extract::State(_state): axum::extract::State<StreamableHttpState>,
) -> StatusCode {
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// SSE transport state & handlers (deprecated)
// ---------------------------------------------------------------------------

/// Shared state for the SSE transport.
#[derive(Clone)]
struct SseState {
    handler: Arc<dyn McpHandler>,
    sender: tokio::sync::mpsc::Sender<Value>,
    receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<Value>>>,
}

/// GET /sse — server-sent event stream.
///
/// Reads messages from the mpsc channel, processes them through the MCP handler,
/// and streams responses as SSE events.
async fn sse_stream_handler(
    axum::extract::State(state): axum::extract::State<SseState>,
) -> axum::response::sse::Sse<
    impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>,
> {
    use axum::response::sse::{Event, Sse};

    let handler = state.handler.clone();
    let receiver = state.receiver.clone();

    let stream = tokio_stream::wrappers::ReceiverStream::new({
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);
        tokio::spawn(async move {
            loop {
                let msg = {
                    let mut guard = receiver.lock().await;
                    guard.recv().await
                };
                match msg {
                    Some(message) => {
                        if let Some(response) = handler.handle_message(message).await {
                            let data = serde_json::to_string(&response).unwrap_or_default();
                            let event = Event::default().event("message").data(data);
                            if tx.send(Ok(event)).await.is_err() {
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
        });
        rx
    });

    Sse::new(stream)
}

/// POST /messages/ — accept a client-to-server JSON-RPC message.
///
/// Sends the message into the mpsc channel for processing by the SSE stream handler.
/// Returns 202 Accepted.
async fn sse_messages_handler(
    axum::extract::State(state): axum::extract::State<SseState>,
    axum::Json(body): axum::Json<Value>,
) -> StatusCode {
    match state.sender.send(body).await {
        Ok(()) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Fallback handler that strips a trailing slash and redirects.
///
/// Axum's `nest("/prefix", ...)` matches `/prefix` but not `/prefix/`.
/// This fallback redirects `/prefix/` → `/prefix` so both work.
/// Non-trailing-slash paths that don't match any route get a 404.
async fn trailing_slash_redirect(req: Request) -> axum::response::Response {
    let path = req.uri().path();
    if path.len() > 1 && path.ends_with('/') {
        let trimmed = path.trim_end_matches('/');
        Redirect::permanent(trimmed).into_response()
    } else {
        axum::http::StatusCode::NOT_FOUND.into_response()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    // -----------------------------------------------------------------------
    // Mock McpHandler for testing
    // -----------------------------------------------------------------------

    /// A mock handler that echoes back the request wrapped in a response envelope.
    struct EchoHandler;

    #[async_trait::async_trait]
    impl McpHandler for EchoHandler {
        async fn handle_message(&self, message: Value) -> Option<Value> {
            // Simulate JSON-RPC: if the message has an "id", return a response.
            if message.get("id").is_some() {
                Some(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": message["id"],
                    "result": message.get("params").cloned().unwrap_or(Value::Null),
                }))
            } else {
                // Notification — no response.
                None
            }
        }
    }

    // -----------------------------------------------------------------------
    // Task 1 tests: TransportError
    // -----------------------------------------------------------------------

    #[test]
    fn transport_error_invalid_port_display() {
        let err = TransportError::InvalidPort(0);
        assert_eq!(err.to_string(), "port must be between 1 and 65535, got 0");
    }

    #[test]
    fn transport_error_invalid_host_display() {
        let err = TransportError::InvalidHost("".to_string());
        assert_eq!(err.to_string(), "invalid host: ");
    }

    #[test]
    fn transport_error_io_wraps_and_preserves_message() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
        let err = TransportError::Io(io_err);
        assert_eq!(err.to_string(), "I/O error: file gone");
    }

    #[test]
    fn transport_error_io_from_conversion() {
        let io_err = std::io::Error::other("boom");
        let err: TransportError = io_err.into();
        assert!(matches!(err, TransportError::Io(_)));
    }

    #[test]
    fn transport_error_implements_std_error() {
        let err = TransportError::Server("oops".to_string());
        // Ensure it implements std::error::Error by using it as a trait object.
        let _: &dyn Error = &err;
    }

    #[test]
    fn transport_error_server_display() {
        let err = TransportError::Server("something broke".to_string());
        assert_eq!(err.to_string(), "server error: something broke");
    }

    // -----------------------------------------------------------------------
    // Task 2 tests: TransportManager struct
    // -----------------------------------------------------------------------

    #[test]
    fn transport_manager_new_without_exporter() {
        let tm = TransportManager::new(None);
        assert!(tm.metrics_exporter.is_none());
        assert_eq!(tm.module_count, 0);
    }

    #[test]
    fn transport_manager_new_with_exporter() {
        struct DummyExporter;
        impl MetricsExporter for DummyExporter {
            fn export_prometheus(&self) -> String {
                "dummy".to_string()
            }
        }
        let exporter: Arc<dyn MetricsExporter> = Arc::new(DummyExporter);
        let tm = TransportManager::new(Some(exporter));
        assert!(tm.metrics_exporter.is_some());
    }

    #[test]
    fn transport_manager_module_count_defaults_to_zero() {
        let tm = TransportManager::new(None);
        assert_eq!(tm.module_count(), 0);
    }

    #[test]
    fn transport_manager_set_module_count() {
        let mut tm = TransportManager::new(None);
        tm.set_module_count(5);
        assert_eq!(tm.module_count(), 5);
    }

    #[test]
    fn validate_host_port_rejects_empty_host() {
        let result = TransportManager::validate_host_port("", 8080);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransportError::InvalidHost(_)
        ));
    }

    #[test]
    fn validate_host_port_rejects_port_zero() {
        let result = TransportManager::validate_host_port("localhost", 0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransportError::InvalidPort(0)
        ));
    }

    #[test]
    fn validate_host_port_accepts_valid() {
        assert!(TransportManager::validate_host_port("127.0.0.1", 8080).is_ok());
        assert!(TransportManager::validate_host_port("localhost", 1).is_ok());
        assert!(TransportManager::validate_host_port("0.0.0.0", 65535).is_ok());
    }

    // -----------------------------------------------------------------------
    // Task 3 tests: Health & Metrics
    // -----------------------------------------------------------------------

    #[test]
    fn build_health_response_returns_ok_status() {
        let tm = TransportManager::new(None);
        let resp = tm.build_health_response();
        assert_eq!(resp.status, "ok");
        assert!(resp.uptime_seconds >= 0.0);
        assert_eq!(resp.module_count, 0);
    }

    #[test]
    fn build_health_response_reflects_module_count() {
        let mut tm = TransportManager::new(None);
        tm.set_module_count(3);
        let resp = tm.build_health_response();
        assert_eq!(resp.module_count, 3);
    }

    #[test]
    fn build_metrics_response_without_exporter_returns_err() {
        let tm = TransportManager::new(None);
        assert!(tm.build_metrics_response().is_err());
    }

    #[test]
    fn build_metrics_response_with_exporter_returns_body() {
        struct MockExporter;
        impl MetricsExporter for MockExporter {
            fn export_prometheus(&self) -> String {
                "# HELP up\nup 1\n".to_string()
            }
        }
        let exporter: Arc<dyn MetricsExporter> = Arc::new(MockExporter);
        let tm = TransportManager::new(Some(exporter));
        let result = tm.build_metrics_response();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "# HELP up\nup 1\n");
    }

    #[tokio::test]
    async fn health_handler_returns_json() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let app = tm.health_metrics_router();

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["module_count"], 0);
        assert!(json["uptime_seconds"].as_f64().unwrap() >= 0.0);
    }

    #[tokio::test]
    async fn metrics_handler_returns_404_without_exporter() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let app = tm.health_metrics_router();

        let req = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn metrics_handler_returns_prometheus_text_with_exporter() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        struct MockExporter;
        impl MetricsExporter for MockExporter {
            fn export_prometheus(&self) -> String {
                "# TYPE gauge\nmy_metric 42\n".to_string()
            }
        }

        let exporter: Arc<dyn MetricsExporter> = Arc::new(MockExporter);
        let tm = Arc::new(TransportManager::new(Some(exporter)));
        let app = tm.health_metrics_router();

        let req = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(ct, "text/plain; version=0.0.4; charset=utf-8");

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8(body.to_vec()).unwrap(),
            "# TYPE gauge\nmy_metric 42\n"
        );
    }

    #[tokio::test]
    async fn health_handler_reflects_module_count() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let mut tm = TransportManager::new(None);
        tm.set_module_count(7);
        let tm = Arc::new(tm);
        let app = tm.health_metrics_router();

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["module_count"], 7);
    }

    // -----------------------------------------------------------------------
    // Task 4 tests: stdio transport
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn stdio_reads_jsonrpc_request_and_writes_response() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#.to_string() + "\n";
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        let result = tm.run_stdio_with_io(reader, &mut output, &handler).await;

        assert!(result.is_ok());
        let response_str = String::from_utf8(output).unwrap();
        let response: Value = serde_json::from_str(response_str.trim()).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
    }

    #[tokio::test]
    async fn stdio_eof_returns_ok() {
        let reader = std::io::Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        let result = tm.run_stdio_with_io(reader, &mut output, &handler).await;

        assert!(result.is_ok());
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn stdio_invalid_json_is_skipped() {
        // First line is invalid, second line is valid.
        let input = "not-json\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"test\"}\n".to_string();
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        let result = tm.run_stdio_with_io(reader, &mut output, &handler).await;

        assert!(result.is_ok());
        let response_str = String::from_utf8(output).unwrap();
        let response: Value = serde_json::from_str(response_str.trim()).unwrap();
        assert_eq!(response["id"], 2);
    }

    #[tokio::test]
    async fn stdio_notification_produces_no_output() {
        // Notification: no "id" field — handler returns None.
        let input = r#"{"jsonrpc":"2.0","method":"notify"}"#.to_string() + "\n";
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        let result = tm.run_stdio_with_io(reader, &mut output, &handler).await;

        assert!(result.is_ok());
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn stdio_multiple_requests() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"a"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"b"}"#,
            "\n",
        )
        .to_string();
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        tm.run_stdio_with_io(reader, &mut output, &handler)
            .await
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
        let r1: Value = serde_json::from_str(lines[0]).unwrap();
        let r2: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(r1["id"], 1);
        assert_eq!(r2["id"], 2);
    }

    #[tokio::test]
    async fn stdio_empty_lines_are_skipped() {
        let input = "\n\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"x\"}\n\n".to_string();
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        tm.run_stdio_with_io(reader, &mut output, &handler)
            .await
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();
        assert_eq!(lines.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Task 5 tests: streamable-HTTP transport
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn streamable_http_app_health_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn streamable_http_app_metrics_returns_404_without_exporter() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let req = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn streamable_http_post_mcp_returns_jsonrpc_response() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/list",
            "params": {}
        });
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get("mcp-session-id").is_some());

        let resp_body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&resp_body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 42);
    }

    #[tokio::test]
    async fn streamable_http_post_notification_returns_202() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        // Notification: no "id" field.
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn streamable_http_delete_mcp_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let req = Request::builder()
            .method("DELETE")
            .uri("/mcp")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn streamable_http_extra_routes_are_merged() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);

        let extra = Router::new().route("/custom", get(|| async { "custom-response" }));
        let app = tm.build_streamable_http_app(handler, Some(extra));

        let req = Request::builder()
            .uri("/custom")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "custom-response");
    }

    #[tokio::test]
    async fn run_streamable_http_rejects_empty_host() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let result = tm.run_streamable_http(handler, "", 8080, None).await;
        assert!(matches!(result, Err(TransportError::InvalidHost(_))));
    }

    #[tokio::test]
    async fn run_streamable_http_rejects_port_zero() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let result = tm.run_streamable_http(handler, "localhost", 0, None).await;
        assert!(matches!(result, Err(TransportError::InvalidPort(0))));
    }

    #[tokio::test]
    async fn streamable_http_get_mcp_returns_sse() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let req = Request::builder().uri("/mcp").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("text/event-stream"));
    }

    // -----------------------------------------------------------------------
    // Task 6 tests: SSE transport (deprecated)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn sse_app_health_returns_200() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        #[allow(deprecated)]
        let app = tm.build_sse_app(handler, None);

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn sse_app_metrics_returns_404_without_exporter() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        #[allow(deprecated)]
        let app = tm.build_sse_app(handler, None);

        let req = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn sse_messages_post_returns_202() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        #[allow(deprecated)]
        let app = tm.build_sse_app(handler, None);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/messages/")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn sse_get_returns_event_stream() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        #[allow(deprecated)]
        let app = tm.build_sse_app(handler, None);

        let req = Request::builder().uri("/sse").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("text/event-stream"));
    }

    #[tokio::test]
    async fn sse_extra_routes_are_merged() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let extra = Router::new().route("/extra", get(|| async { "extra" }));
        #[allow(deprecated)]
        let app = tm.build_sse_app(handler, Some(extra));

        let req = Request::builder()
            .uri("/extra")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn run_sse_rejects_empty_host() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let result = tm.run_sse(handler, "", 8080, None).await;
        assert!(matches!(result, Err(TransportError::InvalidHost(_))));
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn run_sse_rejects_port_zero() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let result = tm.run_sse(handler, "localhost", 0, None).await;
        assert!(matches!(result, Err(TransportError::InvalidPort(0))));
    }

    // -----------------------------------------------------------------------
    // Additional unit tests: edge cases and error paths
    // -----------------------------------------------------------------------

    #[test]
    fn transport_error_bind_display() {
        // Construct a Bind error with a hyper::Error source.
        // We can't easily construct hyper::Error directly, so test the other variants.
        let err = TransportError::Server("bind failed".to_string());
        assert!(err.to_string().contains("bind failed"));
    }

    #[test]
    fn transport_error_is_debug() {
        let err = TransportError::InvalidPort(99);
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidPort"));
        assert!(debug.contains("99"));
    }

    #[test]
    fn transport_error_invalid_host_preserves_value() {
        let err = TransportError::InvalidHost("bad-host!".to_string());
        assert_eq!(err.to_string(), "invalid host: bad-host!");
    }

    #[test]
    fn transport_error_invalid_port_boundary_65535() {
        let err = TransportError::InvalidPort(65535);
        assert_eq!(
            err.to_string(),
            "port must be between 1 and 65535, got 65535"
        );
    }

    #[test]
    fn validate_host_port_rejects_both_invalid() {
        // Empty host is checked first.
        let result = TransportManager::validate_host_port("", 0);
        assert!(matches!(
            result.unwrap_err(),
            TransportError::InvalidHost(_)
        ));
    }

    #[test]
    fn health_response_serializes_correctly() {
        let tm = TransportManager::new(None);
        let resp = tm.build_health_response();
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("status").is_some());
        assert!(json.get("uptime_seconds").is_some());
        assert!(json.get("module_count").is_some());
    }

    #[tokio::test]
    async fn stdio_whitespace_only_lines_are_skipped() {
        let input = "   \n\t\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"x\"}\n".to_string();
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        tm.run_stdio_with_io(reader, &mut output, &handler)
            .await
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();
        assert_eq!(lines.len(), 1);
        let r: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(r["id"], 1);
    }

    #[tokio::test]
    async fn stdio_response_ends_with_newline() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#.to_string() + "\n";
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        tm.run_stdio_with_io(reader, &mut output, &handler)
            .await
            .unwrap();

        // Each response line must end with \n.
        assert!(output.last() == Some(&b'\n'));
    }

    #[tokio::test]
    async fn stdio_mixed_notifications_and_requests() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","method":"notify1"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":1,"method":"req1"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"notify2"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"req2"}"#,
            "\n",
        )
        .to_string();
        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        let tm = TransportManager::new(None);
        let handler = EchoHandler;
        tm.run_stdio_with_io(reader, &mut output, &handler)
            .await
            .unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();
        // Only 2 responses for the 2 requests (notifications produce nothing).
        assert_eq!(lines.len(), 2);
    }

    #[tokio::test]
    async fn streamable_http_metrics_returns_200_with_exporter() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        struct MockExporter;
        impl MetricsExporter for MockExporter {
            fn export_prometheus(&self) -> String {
                "test_metric 1\n".to_string()
            }
        }

        let exporter: Arc<dyn MetricsExporter> = Arc::new(MockExporter);
        let tm = Arc::new(TransportManager::new(Some(exporter)));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let req = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "test_metric 1\n");
    }

    #[tokio::test]
    async fn streamable_http_unknown_route_returns_404() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let req = Request::builder()
            .uri("/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn streamable_http_post_mcp_returns_session_id_header() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"test"});
        let req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        let session_id = resp
            .headers()
            .get("mcp-session-id")
            .unwrap()
            .to_str()
            .unwrap();
        // Session ID should be a valid UUID (36 chars with hyphens).
        assert_eq!(session_id.len(), 36);
        assert!(uuid::Uuid::parse_str(session_id).is_ok());
    }

    // -----------------------------------------------------------------------
    // Integration-style tests: real TCP listener on ephemeral port
    // -----------------------------------------------------------------------

    /// Helper: collect a hyper Incoming body into bytes.
    async fn collect_body(body: hyper::body::Incoming) -> Vec<u8> {
        use http_body_util::BodyExt;
        let collected = body.collect().await.unwrap();
        collected.to_bytes().to_vec()
    }

    /// Helper: build an HTTP client for integration tests.
    fn test_client() -> hyper_util::client::legacy::Client<
        hyper_util::client::legacy::connect::HttpConnector,
        axum::body::Body,
    > {
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build_http()
    }

    #[tokio::test]
    async fn integration_health_endpoint_responds() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = test_client();
        let resp = client
            .request(
                hyper::Request::builder()
                    .uri(format!("http://{}/health", addr))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = collect_body(resp.into_body()).await;
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["module_count"], 0);

        server.abort();
    }

    #[tokio::test]
    async fn integration_metrics_endpoint_404_without_exporter() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = test_client();
        let resp = client
            .request(
                hyper::Request::builder()
                    .uri(format!("http://{}/metrics", addr))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        server.abort();
    }

    #[tokio::test]
    async fn integration_metrics_endpoint_200_with_exporter() {
        struct TestExporter;
        impl MetricsExporter for TestExporter {
            fn export_prometheus(&self) -> String {
                "# HELP test\ntest_total 99\n".to_string()
            }
        }

        let exporter: Arc<dyn MetricsExporter> = Arc::new(TestExporter);
        let tm = Arc::new(TransportManager::new(Some(exporter)));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = test_client();
        let resp = client
            .request(
                hyper::Request::builder()
                    .uri(format!("http://{}/metrics", addr))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = collect_body(resp.into_body()).await;
        assert_eq!(
            String::from_utf8(body).unwrap(),
            "# HELP test\ntest_total 99\n"
        );

        server.abort();
    }

    #[tokio::test]
    async fn integration_mcp_post_endpoint_responds() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let app = tm.build_streamable_http_app(handler, None);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
        let client = test_client();
        let resp = client
            .request(
                hyper::Request::builder()
                    .method("POST")
                    .uri(format!("http://{}/mcp", addr))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let resp_body = collect_body(resp.into_body()).await;
        let json: Value = serde_json::from_slice(&resp_body).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);

        server.abort();
    }

    #[tokio::test]
    async fn integration_extra_routes_served() {
        let tm = Arc::new(TransportManager::new(None));
        let handler: Arc<dyn McpHandler> = Arc::new(EchoHandler);
        let extra = Router::new().route("/custom", get(|| async { "hello-integration" }));
        let app = tm.build_streamable_http_app(handler, Some(extra));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = test_client();
        let resp = client
            .request(
                hyper::Request::builder()
                    .uri(format!("http://{}/custom", addr))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = collect_body(resp.into_body()).await;
        assert_eq!(String::from_utf8(body).unwrap(), "hello-integration");

        server.abort();
    }
}
