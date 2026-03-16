//! MCPServer — the main MCP server struct.
//!
//! Combines the factory, router, transport, and listener into a single
//! server lifecycle.


use std::collections::HashSet;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::server::types::{
    CallToolResult, ReadResourceContents, Resource, Tool,
};

// ---------------------------------------------------------------------------
// TransportKind
// ---------------------------------------------------------------------------

/// The transport protocol used by the MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    /// Standard I/O transport (stdin/stdout).
    Stdio,
    /// Streamable HTTP transport.
    StreamableHttp,
    /// Server-Sent Events transport.
    Sse,
}

/// Error returned when parsing an invalid transport string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown transport: \"{0}\"")]
pub struct ParseTransportError(String);

impl FromStr for TransportKind {
    type Err = ParseTransportError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "stdio" => Ok(Self::Stdio),
            "streamable-http" => Ok(Self::StreamableHttp),
            "sse" => Ok(Self::Sse),
            _ => Err(ParseTransportError(s.to_string())),
        }
    }
}

impl fmt::Display for TransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdio => write!(f, "stdio"),
            Self::StreamableHttp => write!(f, "streamable-http"),
            Self::Sse => write!(f, "sse"),
        }
    }
}

impl TransportKind {
    /// Return the address string for this transport.
    ///
    /// * `Stdio` always returns `"stdio"` (host/port are ignored).
    /// * `StreamableHttp` and `Sse` return `"http://{host}:{port}"`.
    pub fn address(&self, host: &str, port: u16) -> String {
        match self {
            Self::Stdio => "stdio".to_string(),
            Self::StreamableHttp | Self::Sse => format!("http://{}:{}", host, port),
        }
    }
}

// ---------------------------------------------------------------------------
// MCPServerConfig
// ---------------------------------------------------------------------------

/// Configuration for constructing an [`MCPServer`].
///
/// Defaults match the Python `MCPServer.__init__` defaults.
#[derive(Debug, Clone)]
pub struct MCPServerConfig {
    /// Transport protocol.
    pub transport: TransportKind,
    /// Bind host for network transports.
    pub host: String,
    /// Bind port for network transports.
    pub port: u16,
    /// Server name advertised in MCP init.
    pub name: String,
    /// Server version string.
    pub version: Option<String>,
    /// Whether to validate tool inputs against their JSON schema.
    pub validate_inputs: bool,
    /// Optional tags to filter which modules are exposed as tools.
    pub tags: Option<Vec<String>>,
    /// Optional prefix to filter which modules are exposed as tools.
    pub prefix: Option<String>,
    /// Whether authentication is required for HTTP transports.
    pub require_auth: bool,
    /// Paths exempt from authentication.
    pub exempt_paths: Option<HashSet<String>>,
    // NOTE: authenticator and metrics_collector are trait-object fields.
    // They will be added when their trait definitions are available.
    // pub authenticator: Option<Arc<dyn Authenticator>>,
    // pub metrics_collector: Option<Arc<dyn MetricsExporter>>,
}

impl Default for MCPServerConfig {
    fn default() -> Self {
        Self {
            transport: TransportKind::Stdio,
            host: "127.0.0.1".to_string(),
            port: 8000,
            name: "apcore-mcp".to_string(),
            version: None,
            validate_inputs: false,
            tags: None,
            prefix: None,
            require_auth: true,
            exempt_paths: None,
        }
    }
}

// ---------------------------------------------------------------------------
// FactoryError
// ---------------------------------------------------------------------------

/// Error type for factory/handler operations.
#[derive(Debug, thiserror::Error)]
pub enum FactoryError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    #[error("Unsupported URI scheme: {0}")]
    UnsupportedScheme(String),
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// CallToolHandler
// ---------------------------------------------------------------------------

/// Type alias for the async call_tool handler.
pub type CallToolHandler = Arc<
    dyn Fn(String, Value, Option<Value>) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// RegistryOrExecutor
// ---------------------------------------------------------------------------

/// Input to [`MCPServer`]: either a registry or an executor.
///
/// The exact inner types are left as opaque trait objects so that the server
/// module does not depend on concrete `apcore` types directly.  Placeholder
/// types (`()`) are used until the `apcore` crate exposes the real traits.
#[derive(Debug, Clone)]
pub enum RegistryOrExecutor {
    /// An apcore Registry (owns both registry data and an executor).
    Registry(Arc<dyn std::any::Any + Send + Sync>),
    /// A standalone Executor.
    Executor(Arc<dyn std::any::Any + Send + Sync>),
}

// ---------------------------------------------------------------------------
// MCPServer
// ---------------------------------------------------------------------------

/// The MCP server. Created by [`MCPServerFactory`](super::factory::MCPServerFactory).
pub struct MCPServer {
    config: MCPServerConfig,

    /// Optional registry-or-executor input (not yet wired to factory/router).
    registry_or_executor: Option<RegistryOrExecutor>,

    // --- Handler storage ---
    /// Handler for `list_tools` requests.
    pub(crate) list_tools_handler: Option<Arc<dyn Fn() -> Vec<Tool> + Send + Sync>>,
    /// Handler for `call_tool` requests.
    pub(crate) call_tool_handler: Option<CallToolHandler>,
    /// Handler for `list_resources` requests.
    pub(crate) list_resources_handler: Option<Arc<dyn Fn() -> Vec<Resource> + Send + Sync>>,
    /// Handler for `read_resource` requests.
    pub(crate) read_resource_handler:
        Option<Arc<dyn Fn(String) -> Result<Vec<ReadResourceContents>, FactoryError> + Send + Sync>>,

    // --- Lifecycle state ---
    /// Handle for the spawned server task.
    join_handle: Option<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>>,
    /// Sender side of the shutdown watch channel.
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl MCPServer {
    /// Create a new MCP server from a configuration.
    pub fn new(config: MCPServerConfig) -> Self {
        Self {
            config,
            registry_or_executor: None,
            list_tools_handler: None,
            call_tool_handler: None,
            list_resources_handler: None,
            read_resource_handler: None,
            join_handle: None,
            shutdown_tx: None,
        }
    }

    /// Create a new MCP server from a [`RegistryOrExecutor`] and configuration.
    pub fn with_registry_or_executor(
        registry_or_executor: RegistryOrExecutor,
        config: MCPServerConfig,
    ) -> Self {
        Self {
            config,
            registry_or_executor: Some(registry_or_executor),
            list_tools_handler: None,
            call_tool_handler: None,
            list_resources_handler: None,
            read_resource_handler: None,
            join_handle: None,
            shutdown_tx: None,
        }
    }

    /// Create a new MCP server with individual parameters (legacy API).
    ///
    /// Prefer [`MCPServer::new`] with [`MCPServerConfig`] for new code.
    pub fn with_params(name: &str, transport: &str, host: &str, port: u16) -> Self {
        let transport_kind = transport.parse::<TransportKind>().unwrap_or(TransportKind::Stdio);
        let config = MCPServerConfig {
            name: name.to_string(),
            transport: transport_kind,
            host: host.to_string(),
            port,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Returns the server name.
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Returns the transport kind.
    pub fn transport(&self) -> TransportKind {
        self.config.transport
    }

    /// Returns a reference to the server configuration.
    pub fn config(&self) -> &MCPServerConfig {
        &self.config
    }

    /// Returns true if tool handlers have been registered.
    pub fn has_tool_handlers(&self) -> bool {
        self.list_tools_handler.is_some() && self.call_tool_handler.is_some()
    }

    /// Returns true if resource handlers have been registered.
    pub fn has_resource_handlers(&self) -> bool {
        self.list_resources_handler.is_some() && self.read_resource_handler.is_some()
    }

    /// Invoke the list_tools handler if registered.
    pub fn list_tools(&self) -> Option<Vec<Tool>> {
        self.list_tools_handler.as_ref().map(|h| h())
    }

    /// Invoke the call_tool handler if registered.
    pub fn call_tool(
        &self,
        name: String,
        arguments: Value,
        extra: Option<Value>,
    ) -> Option<Pin<Box<dyn Future<Output = CallToolResult> + Send>>> {
        self.call_tool_handler.as_ref().map(|h| h(name, arguments, extra))
    }

    /// Invoke the list_resources handler if registered.
    pub fn list_resources(&self) -> Option<Vec<Resource>> {
        self.list_resources_handler.as_ref().map(|h| h())
    }

    /// Invoke the read_resource handler if registered.
    pub fn read_resource(&self, uri: String) -> Option<Result<Vec<ReadResourceContents>, FactoryError>> {
        self.read_resource_handler.as_ref().map(|h| h(uri))
    }

    /// Returns true if the server task is currently running.
    pub fn is_running(&self) -> bool {
        self.join_handle.is_some()
    }

    /// Returns a reference to the registry-or-executor, if one was provided.
    pub fn registry_or_executor(&self) -> Option<&RegistryOrExecutor> {
        self.registry_or_executor.as_ref()
    }

    /// Start the server (spawns the transport loop).
    ///
    /// This is idempotent: calling `start()` on an already-running server is
    /// a no-op and returns `Ok(())`.
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Idempotent: already started.
        if self.join_handle.is_some() {
            return Ok(());
        }

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let (started_tx, started_rx) = tokio::sync::oneshot::channel::<()>();

        let handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
            tokio::spawn(async move {
                // Signal that the task has started.
                let _ = started_tx.send(());

                // Wait for shutdown signal.
                // In the future this will run the actual transport loop via
                // `tokio::select!` racing the transport future against the
                // shutdown channel.  For now the task simply awaits shutdown.
                let _ = shutdown_rx.changed().await;

                Ok(())
            });

        self.join_handle = Some(handle);

        // Wait for the spawned task to signal it has started (with timeout).
        tokio::time::timeout(std::time::Duration::from_secs(10), started_rx)
            .await
            .map_err(|_| "server start timed out")?
            .map_err(|_| "server start channel dropped")?;

        Ok(())
    }

    /// Wait for the server to shut down.
    ///
    /// If the server has not been started, this returns immediately.
    pub async fn wait(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(handle) = self.join_handle.take() {
            handle
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
        }
        Ok(())
    }

    /// Gracefully stop the server.
    ///
    /// If the server has not been started, this is a no-op.
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(true);
        }
        // Wait for the spawned task to finish.
        if let Some(handle) = self.join_handle.take() {
            handle
                .await
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
        }
        self.shutdown_tx = None;
        Ok(())
    }

    /// The address the server is listening on.
    ///
    /// Delegates to [`TransportKind::address`].
    pub fn address(&self) -> String {
        self.config.transport.address(&self.config.host, self.config.port)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- TransportKind::from_str tests ----

    #[test]
    fn transport_kind_from_str_stdio() {
        assert_eq!("stdio".parse::<TransportKind>().unwrap(), TransportKind::Stdio);
    }

    #[test]
    fn transport_kind_from_str_streamable_http() {
        assert_eq!(
            "streamable-http".parse::<TransportKind>().unwrap(),
            TransportKind::StreamableHttp
        );
    }

    #[test]
    fn transport_kind_from_str_sse() {
        assert_eq!("sse".parse::<TransportKind>().unwrap(), TransportKind::Sse);
    }

    #[test]
    fn transport_kind_from_str_case_insensitive() {
        assert_eq!("STDIO".parse::<TransportKind>().unwrap(), TransportKind::Stdio);
        assert_eq!("Streamable-Http".parse::<TransportKind>().unwrap(), TransportKind::StreamableHttp);
        assert_eq!("SSE".parse::<TransportKind>().unwrap(), TransportKind::Sse);
    }

    #[test]
    fn transport_kind_from_str_unknown_returns_err() {
        let result = "unknown".parse::<TransportKind>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "unknown transport: \"unknown\""
        );
    }

    // ---- TransportKind::address tests ----

    #[test]
    fn transport_kind_stdio_address() {
        assert_eq!(TransportKind::Stdio.address("127.0.0.1", 8000), "stdio");
    }

    #[test]
    fn transport_kind_streamable_http_address() {
        assert_eq!(
            TransportKind::StreamableHttp.address("127.0.0.1", 8000),
            "http://127.0.0.1:8000"
        );
    }

    #[test]
    fn transport_kind_sse_address() {
        assert_eq!(
            TransportKind::Sse.address("0.0.0.0", 9090),
            "http://0.0.0.0:9090"
        );
    }

    // ---- TransportKind Display tests ----

    #[test]
    fn transport_kind_display() {
        assert_eq!(TransportKind::Stdio.to_string(), "stdio");
        assert_eq!(TransportKind::StreamableHttp.to_string(), "streamable-http");
        assert_eq!(TransportKind::Sse.to_string(), "sse");
    }

    // ---- MCPServerConfig default tests ----

    #[test]
    fn config_default_values() {
        let config = MCPServerConfig::default();
        assert_eq!(config.transport, TransportKind::Stdio);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8000);
        assert_eq!(config.name, "apcore-mcp");
        assert_eq!(config.version, None);
        assert_eq!(config.validate_inputs, false);
        assert_eq!(config.require_auth, true);
        assert_eq!(config.tags, None);
        assert_eq!(config.prefix, None);
        assert_eq!(config.exempt_paths, None);
    }

    #[test]
    fn config_can_be_customized() {
        let config = MCPServerConfig {
            transport: TransportKind::StreamableHttp,
            host: "0.0.0.0".to_string(),
            port: 9090,
            name: "my-server".to_string(),
            version: Some("1.0.0".to_string()),
            validate_inputs: true,
            require_auth: false,
            tags: Some(vec!["tag1".to_string()]),
            prefix: Some("my_prefix".to_string()),
            exempt_paths: Some(HashSet::from(["/_health".to_string()])),
        };
        assert_eq!(config.transport, TransportKind::StreamableHttp);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9090);
        assert_eq!(config.name, "my-server");
        assert_eq!(config.version, Some("1.0.0".to_string()));
        assert_eq!(config.validate_inputs, true);
        assert_eq!(config.require_auth, false);
        assert_eq!(config.tags.as_ref().unwrap().len(), 1);
        assert_eq!(config.prefix, Some("my_prefix".to_string()));
        assert!(config.exempt_paths.as_ref().unwrap().contains("/_health"));
    }

    // ---- MCPServer with config tests ----

    #[test]
    fn server_new_with_config() {
        let config = MCPServerConfig {
            name: "test-server".to_string(),
            transport: TransportKind::StreamableHttp,
            host: "0.0.0.0".to_string(),
            port: 9090,
            ..Default::default()
        };
        let server = MCPServer::new(config);
        assert_eq!(server.name(), "test-server");
        assert_eq!(server.transport(), TransportKind::StreamableHttp);
        assert_eq!(server.address(), "http://0.0.0.0:9090");
    }

    #[test]
    fn server_with_params_legacy_api() {
        let server = MCPServer::with_params("test", "stdio", "127.0.0.1", 0);
        assert_eq!(server.name(), "test");
        assert_eq!(server.transport(), TransportKind::Stdio);
        assert_eq!(server.address(), "stdio");
    }

    #[test]
    fn server_address_delegates_to_transport_kind() {
        let stdio = MCPServer::new(MCPServerConfig::default());
        assert_eq!(stdio.address(), "stdio");

        let http = MCPServer::new(MCPServerConfig {
            transport: TransportKind::StreamableHttp,
            host: "localhost".to_string(),
            port: 3000,
            ..Default::default()
        });
        assert_eq!(http.address(), "http://localhost:3000");
    }

    #[test]
    fn server_config_accessor() {
        let config = MCPServerConfig {
            validate_inputs: true,
            require_auth: false,
            ..Default::default()
        };
        let server = MCPServer::new(config);
        assert_eq!(server.config().validate_inputs, true);
        assert_eq!(server.config().require_auth, false);
    }

    // ---- Task 4: RegistryOrExecutor and struct completeness ----

    #[test]
    fn registry_or_executor_registry_variant() {
        let val: Arc<dyn std::any::Any + Send + Sync> = Arc::new(42u32);
        let roe = RegistryOrExecutor::Registry(val);
        assert!(matches!(roe, RegistryOrExecutor::Registry(_)));
    }

    #[test]
    fn registry_or_executor_executor_variant() {
        let val: Arc<dyn std::any::Any + Send + Sync> = Arc::new("executor");
        let roe = RegistryOrExecutor::Executor(val);
        assert!(matches!(roe, RegistryOrExecutor::Executor(_)));
    }

    #[test]
    fn server_with_registry_or_executor_stores_it() {
        let val: Arc<dyn std::any::Any + Send + Sync> = Arc::new(42u32);
        let roe = RegistryOrExecutor::Registry(val);
        let server = MCPServer::with_registry_or_executor(roe, MCPServerConfig::default());
        assert!(server.registry_or_executor().is_some());
        assert!(matches!(
            server.registry_or_executor().unwrap(),
            RegistryOrExecutor::Registry(_)
        ));
    }

    #[test]
    fn server_new_has_no_registry_or_executor() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(server.registry_or_executor().is_none());
    }

    #[test]
    fn server_not_running_after_construction() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(!server.is_running());
        assert!(server.join_handle.is_none());
        assert!(server.shutdown_tx.is_none());
    }

    #[test]
    fn server_new_with_stdio_address() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert_eq!(server.address(), "stdio");
    }

    #[test]
    fn server_new_with_streamable_http_address() {
        let config = MCPServerConfig {
            transport: TransportKind::StreamableHttp,
            ..Default::default()
        };
        let server = MCPServer::new(config);
        assert_eq!(server.address(), "http://127.0.0.1:8000");
    }

    #[test]
    fn server_new_with_custom_host_port_address() {
        let config = MCPServerConfig {
            transport: TransportKind::StreamableHttp,
            host: "10.0.0.1".to_string(),
            port: 3000,
            ..Default::default()
        };
        let server = MCPServer::new(config);
        assert_eq!(server.address(), "http://10.0.0.1:3000");
    }

    // ---- Task 5: Server lifecycle tests ----

    #[tokio::test]
    async fn start_is_idempotent() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.start().await.unwrap();
        assert!(server.is_running());
        // Second start is a no-op.
        server.start().await.unwrap();
        assert!(server.is_running());
        // Clean up.
        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn stop_on_unstarted_server_is_noop() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        assert!(!server.is_running());
        // Should not panic or error.
        server.stop().await.unwrap();
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn start_then_stop_does_not_panic() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.start().await.unwrap();
        assert!(server.is_running());
        server.stop().await.unwrap();
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn after_stop_wait_completes() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.start().await.unwrap();
        server.stop().await.unwrap();
        // wait on a stopped server should return immediately.
        server.wait().await.unwrap();
    }

    #[tokio::test]
    async fn wait_on_unstarted_server_returns_immediately() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.wait().await.unwrap();
    }

    #[tokio::test]
    async fn stop_then_is_running_false() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.start().await.unwrap();
        assert!(server.is_running());
        server.stop().await.unwrap();
        assert!(!server.is_running());
        assert!(server.shutdown_tx.is_none());
    }

    // ---- Handler tests ----

    #[test]
    fn has_tool_handlers_false_by_default() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(!server.has_tool_handlers());
    }

    #[test]
    fn has_resource_handlers_false_by_default() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(!server.has_resource_handlers());
    }

    #[test]
    fn has_tool_handlers_true_when_both_set() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.list_tools_handler = Some(Arc::new(|| vec![]));
        // Only list_tools set — should still be false
        assert!(!server.has_tool_handlers());

        server.call_tool_handler = Some(Arc::new(|_name, _args, _extra| {
            Box::pin(async {
                CallToolResult { content: vec![], is_error: false }
            })
        }));
        assert!(server.has_tool_handlers());
    }

    #[test]
    fn has_resource_handlers_true_when_both_set() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.list_resources_handler = Some(Arc::new(|| vec![]));
        assert!(!server.has_resource_handlers());

        server.read_resource_handler = Some(Arc::new(|_uri| Ok(vec![])));
        assert!(server.has_resource_handlers());
    }

    #[test]
    fn list_tools_returns_none_when_no_handler() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(server.list_tools().is_none());
    }

    #[test]
    fn list_tools_returns_tools_from_handler() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.list_tools_handler = Some(Arc::new(|| {
            vec![Tool {
                name: "test-tool".to_string(),
                description: "A test tool".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
                annotations: None,
                meta: None,
            }]
        }));
        let tools = server.list_tools().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test-tool");
    }

    #[tokio::test]
    async fn call_tool_returns_none_when_no_handler() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(server.call_tool("foo".into(), Value::Null, None).is_none());
    }

    #[tokio::test]
    async fn call_tool_invokes_handler() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.call_tool_handler = Some(Arc::new(|_name, _args, _extra| {
            Box::pin(async move {
                CallToolResult { content: vec![], is_error: false }
            })
        }));
        let fut = server.call_tool("my-tool".into(), Value::Null, None);
        assert!(fut.is_some());
        let result = fut.unwrap().await;
        assert!(!result.is_error);
    }

    #[test]
    fn list_resources_returns_none_when_no_handler() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(server.list_resources().is_none());
    }

    #[test]
    fn read_resource_returns_none_when_no_handler() {
        let server = MCPServer::new(MCPServerConfig::default());
        assert!(server.read_resource("test://uri".into()).is_none());
    }

    // ---- Integration: server start/stop lifecycle with address checks ----

    #[tokio::test]
    async fn address_consistent_before_and_after_start() {
        let config = MCPServerConfig {
            transport: TransportKind::StreamableHttp,
            host: "0.0.0.0".to_string(),
            port: 9090,
            ..Default::default()
        };
        let mut server = MCPServer::new(config);
        let addr_before = server.address();
        server.start().await.unwrap();
        let addr_during = server.address();
        server.stop().await.unwrap();
        let addr_after = server.address();
        assert_eq!(addr_before, "http://0.0.0.0:9090");
        assert_eq!(addr_before, addr_during);
        assert_eq!(addr_during, addr_after);
    }

    #[tokio::test]
    async fn full_lifecycle_start_stop_wait() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        assert!(!server.is_running());

        server.start().await.unwrap();
        assert!(server.is_running());

        server.stop().await.unwrap();
        assert!(!server.is_running());

        // wait after stop is safe
        server.wait().await.unwrap();
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn double_stop_is_safe() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.start().await.unwrap();
        server.stop().await.unwrap();
        // Second stop should not panic or error
        server.stop().await.unwrap();
    }

    #[tokio::test]
    async fn restart_after_stop() {
        let mut server = MCPServer::new(MCPServerConfig::default());
        server.start().await.unwrap();
        assert!(server.is_running());
        server.stop().await.unwrap();
        assert!(!server.is_running());

        // Restart should work
        server.start().await.unwrap();
        assert!(server.is_running());
        server.stop().await.unwrap();
    }

    // ---- TransportKind edge cases ----

    #[test]
    fn transport_kind_from_str_empty_string_is_error() {
        assert!("".parse::<TransportKind>().is_err());
    }

    #[test]
    fn parse_transport_error_display() {
        let err = ParseTransportError("bad".to_string());
        assert_eq!(err.to_string(), "unknown transport: \"bad\"");
    }

    #[test]
    fn transport_kind_clone_and_copy() {
        let t = TransportKind::Sse;
        let t2 = t; // Copy
        let t3 = t.clone();
        assert_eq!(t, t2);
        assert_eq!(t, t3);
    }

    // ---- FactoryError tests ----

    #[test]
    fn factory_error_display() {
        let e1 = FactoryError::ResourceNotFound("foo".into());
        assert_eq!(e1.to_string(), "Resource not found: foo");

        let e2 = FactoryError::UnsupportedScheme("bar".into());
        assert_eq!(e2.to_string(), "Unsupported URI scheme: bar");

        let e3 = FactoryError::Other("something".into());
        assert_eq!(e3.to_string(), "something");
    }

    #[test]
    fn with_params_falls_back_to_stdio_for_unknown_transport() {
        let server = MCPServer::with_params("test", "invalid-transport", "1.2.3.4", 5555);
        assert_eq!(server.transport(), TransportKind::Stdio);
        assert_eq!(server.address(), "stdio");
    }
}
