//! APCoreMCP — unified bridge class.
//!
//! Provides builder-pattern construction and the main `serve` / `async_serve`
//! entry points that wire an apcore registry+executor to an MCP server.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use serde_json::Value;

use apcore::approval::ApprovalHandler;
use apcore::executor::Executor;
use apcore::registry::registry::Registry;

use crate::auth::protocol::Authenticator;
use crate::converters::openai::OpenAIConverter;
use crate::explorer::{create_explorer_mount, ExplorerConfig, ToolInfo};
use crate::server::factory::MCPServerFactory;
use crate::server::router::{ExecutionRouter, ExecutorError, OutputFormatter};
use crate::server::server::{MCPServer, ServerHandler};
use crate::server::transport::{MetricsExporter, TransportManager};
use crate::server::types::{InitializationOptions, Tool};

// ---- BackendSource enum -----------------------------------------------------

/// Describes the source of the apcore backend.
///
/// This replaces the Python polymorphic `str | Path | Registry | Executor`
/// constructor parameter with a typed enum.
///
/// # Examples
///
/// ```
/// use apcore_mcp::apcore_mcp::BackendSource;
/// use std::path::PathBuf;
///
/// // From a string path
/// let source: BackendSource = "./extensions".into();
///
/// // From a PathBuf
/// let source: BackendSource = PathBuf::from("./extensions").into();
/// ```
#[derive(Debug)]
pub enum BackendSource {
    /// Path to an extensions directory. A [`Registry`] will be created
    /// and [`discover()`](Registry::discover) called automatically.
    ExtensionsDir(PathBuf),
    /// A pre-built [`Registry`] instance.
    Registry(Arc<Registry>),
    /// A pre-built [`Executor`] instance (which contains its own registry).
    Executor(Arc<Executor>),
}

impl From<&str> for BackendSource {
    fn from(s: &str) -> Self {
        BackendSource::ExtensionsDir(PathBuf::from(s))
    }
}

impl From<String> for BackendSource {
    fn from(s: String) -> Self {
        BackendSource::ExtensionsDir(PathBuf::from(s))
    }
}

impl From<PathBuf> for BackendSource {
    fn from(p: PathBuf) -> Self {
        BackendSource::ExtensionsDir(p)
    }
}

impl From<Arc<Registry>> for BackendSource {
    fn from(r: Arc<Registry>) -> Self {
        BackendSource::Registry(r)
    }
}

impl From<Arc<Executor>> for BackendSource {
    fn from(e: Arc<Executor>) -> Self {
        BackendSource::Executor(e)
    }
}

// ---- APCoreMCPError ---------------------------------------------------------

/// Errors that can occur during APCoreMCP construction or operation.
#[derive(Debug, thiserror::Error)]
pub enum APCoreMCPError {
    /// The server name must not be empty.
    #[error("name must not be empty")]
    EmptyName,

    /// The server name exceeds the maximum length of 255 characters.
    #[error("name exceeds maximum length of 255: {0}")]
    NameTooLong(usize),

    /// Tag values must not be empty strings.
    #[error("tag values must not be empty")]
    EmptyTag,

    /// The prefix must not be empty when provided.
    #[error("prefix must not be empty")]
    EmptyPrefix,

    /// The specified log level is not recognized.
    #[error("unknown log level: {0:?}. Valid: [\"CRITICAL\", \"DEBUG\", \"ERROR\", \"INFO\", \"WARNING\"]")]
    InvalidLogLevel(String),

    /// The explorer prefix must start with '/'.
    #[error("explorer_prefix must start with '/'")]
    InvalidExplorerPrefix,

    /// Failed to resolve the backend (registry/executor) from the source.
    #[error("backend resolution failed: {0}")]
    BackendResolution(String),

    /// Unknown transport type.
    #[error("Unknown transport: {0:?}. Expected 'stdio', 'streamable-http', or 'sse'.")]
    UnknownTransport(String),

    /// A server runtime error.
    #[error("server error: {0}")]
    ServerError(String),

    /// OpenAI conversion error.
    #[error("openai conversion error: {0}")]
    ConverterError(#[from] crate::converters::openai::ConverterError),
}

// ---- APCoreMCPConfig --------------------------------------------------------

/// Configuration for the APCoreMCP bridge.
///
/// Matches the Python `APCoreMCP.__init__` parameters. Use [`Default::default()`]
/// for sensible defaults, then override fields as needed.
#[derive(Debug, Clone)]
pub struct APCoreMCPConfig {
    /// MCP server name (max 255 chars).
    pub name: String,
    /// MCP server version. `None` defaults to the crate version.
    pub version: Option<String>,
    /// Transport type: "stdio", "streamable-http", or "sse".
    pub transport: String,
    /// Host address for HTTP-based transports.
    pub host: String,
    /// Port number for HTTP-based transports.
    pub port: u16,
    /// Filter modules by tags. Only modules with ALL specified tags are exposed.
    pub tags: Option<Vec<String>>,
    /// Filter modules by ID prefix.
    pub prefix: Option<String>,
    /// Log level for the apcore_mcp logger (e.g. "DEBUG", "INFO").
    pub log_level: Option<String>,
    /// Validate tool inputs against schemas before execution.
    pub validate_inputs: bool,
    /// If true, unauthenticated requests receive 401.
    pub require_auth: bool,
    /// Exact paths that bypass authentication.
    pub exempt_paths: Option<HashSet<String>>,
    /// Enable the browser-based Tool Explorer UI (HTTP only).
    pub explorer: bool,
    /// URL prefix for the explorer.
    pub explorer_prefix: String,
    /// Page title shown in the explorer browser tab and heading.
    pub explorer_title: String,
    /// Optional project name shown in the explorer footer.
    pub explorer_project_name: Option<String>,
    /// Optional project URL linked in the explorer footer.
    pub explorer_project_url: Option<String>,
    /// Allow tool execution from the explorer UI.
    pub allow_execute: bool,
}

impl Default for APCoreMCPConfig {
    fn default() -> Self {
        Self {
            name: "apcore-mcp".to_string(),
            version: None,
            transport: "stdio".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            tags: None,
            prefix: None,
            log_level: None,
            validate_inputs: false,
            require_auth: true,
            exempt_paths: None,
            explorer: false,
            explorer_prefix: "/explorer".to_string(),
            explorer_title: "APCore MCP Explorer".to_string(),
            explorer_project_name: Some("apcore-mcp".to_string()),
            explorer_project_url: Some(
                "https://github.com/aiperceivable/apcore-mcp-rust".to_string(),
            ),
            allow_execute: false,
        }
    }
}

// ---- Valid log levels -------------------------------------------------------

const VALID_LOG_LEVELS: &[&str] = &["CRITICAL", "DEBUG", "ERROR", "INFO", "WARNING"];

// ---- Executor adapter -------------------------------------------------------

/// Adapts `apcore::Executor` to the [`crate::server::router::Executor`] trait
/// so that `ExecutionRouter` can dispatch MCP tool calls through it.
struct ApcoreExecutorAdapter {
    inner: Arc<Executor>,
}

#[async_trait::async_trait]
impl crate::server::router::Executor for ApcoreExecutorAdapter {
    async fn call_async(
        &self,
        module_id: &str,
        inputs: &Value,
        _context: Option<&Value>,
    ) -> Result<Value, ExecutorError> {
        self.inner
            .call_async(module_id, inputs.clone(), None, None)
            .await
            .map_err(|e| ExecutorError::Execution {
                code: format!("{:?}", e.code),
                message: e.to_string(),
                details: None,
            })
    }
}

// ---- APCoreMCP struct -------------------------------------------------------

/// The main MCP bridge. Wraps an apcore registry and executor, exposing
/// them as MCP tools over the configured transport.
pub struct APCoreMCP {
    config: APCoreMCPConfig,
    /// User-supplied registry for tool discovery. `None` when backend is
    /// an Executor (tool discovery goes through `executor.registry()`).
    standalone_registry: Option<Arc<Registry>>,
    executor: Arc<Executor>,
    /// Optional authenticator for HTTP transport auth middleware.
    authenticator: Option<Arc<dyn Authenticator>>,
    metrics_collector: Option<Arc<dyn MetricsExporter>>,
    /// Reserved — accepted by builder but not yet passed to
    /// `ExecutionRouter::new_with_formatter` (requires converting to
    /// `Arc<dyn Fn>` or taking `&mut self`).
    #[allow(dead_code)]
    output_formatter: Option<OutputFormatter>,
    /// Reserved — accepted by builder but not yet passed to the executor
    /// pipeline (requires `resolve_executor` to support post-construction
    /// injection).
    #[allow(dead_code)]
    approval_handler: Option<Arc<dyn ApprovalHandler>>,
}

impl APCoreMCP {
    /// Create a new builder.
    pub fn builder() -> APCoreMCPBuilder {
        APCoreMCPBuilder::default()
    }

    // -- Property accessors ---------------------------------------------------

    /// Internal helper — returns the registry for tool discovery.
    fn reg(&self) -> &Registry {
        match &self.standalone_registry {
            Some(reg) => reg,
            None => self.executor.registry(),
        }
    }

    /// Returns a reference to the underlying registry.
    pub fn registry(&self) -> &Registry {
        self.reg()
    }

    /// Returns a reference to the underlying executor.
    pub fn executor(&self) -> &Arc<Executor> {
        &self.executor
    }

    /// Returns the currently registered tool names (module IDs), filtered
    /// by the configured tags and prefix.
    pub fn tools(&self) -> Vec<String> {
        let tags_refs: Option<Vec<&str>> = self
            .config
            .tags
            .as_ref()
            .map(|t| t.iter().map(|s| s.as_str()).collect());
        let tags_slice: Option<&[&str]> = tags_refs.as_deref();
        let prefix = self.config.prefix.as_deref();
        self.reg()
            .list(tags_slice, prefix)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    // -- build_server_components ----------------------------------------------

    /// Build the shared MCP server components used by `serve()` and `async_serve()`.
    ///
    /// Returns `(MCPServer, ExecutionRouter, Vec<Tool>, InitializationOptions, version)`.
    #[allow(clippy::type_complexity)]
    pub(crate) fn build_server_components(
        &self,
    ) -> Result<
        (
            MCPServer,
            Arc<ExecutionRouter>,
            Vec<Tool>,
            InitializationOptions,
            String,
        ),
        APCoreMCPError,
    > {
        let version = self
            .config
            .version
            .clone()
            .unwrap_or_else(|| crate::VERSION.to_string());

        let factory = MCPServerFactory::new();
        let mut server = factory.create_server(&self.config.name, &version);

        // Build filtered tool definitions
        let tags_refs: Option<Vec<&str>> = self
            .config
            .tags
            .as_ref()
            .map(|t| t.iter().map(|s| s.as_str()).collect());
        let tags_slice: Option<&[&str]> = tags_refs.as_deref();
        let prefix = self.config.prefix.as_deref();
        let tools = factory.build_tools(self.reg(), tags_slice, prefix);

        // Create execution router backed by the real apcore Executor.
        let adapter = ApcoreExecutorAdapter {
            inner: Arc::clone(&self.executor),
        };
        let router = Arc::new(ExecutionRouter::new(
            Box::new(adapter),
            self.config.validate_inputs,
            None,
        ));

        // Register handlers
        factory.register_handlers(&mut server, tools.clone(), Arc::clone(&router));

        // Register resource handlers
        factory.register_resource_handlers(&mut server, self.reg());

        // Build init options
        let init_options = factory.build_init_options(&server, &self.config.name, &version);

        Ok((server, router, tools, init_options, version))
    }

    /// Build an [`ExplorerConfig`] from the given tools and explorer parameters.
    #[allow(clippy::too_many_arguments)]
    fn build_explorer_config(
        &self,
        tools: &[Tool],
        router: &Arc<ExecutionRouter>,
        prefix: &str,
        allow_execute: bool,
        title: &str,
        project_name: Option<&str>,
        project_url: Option<&str>,
    ) -> ExplorerConfig {
        let tool_infos: Vec<ToolInfo> = tools
            .iter()
            .map(|t| ToolInfo {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect();

        let mut config = ExplorerConfig::new(tool_infos)
            .explorer_prefix(prefix)
            .allow_execute(allow_execute)
            .title(title);

        if let Some(name) = project_name {
            config = config.project_name(name);
        }
        if let Some(url) = project_url {
            config = config.project_url(url);
        }

        // Wire handle_call if allow_execute is enabled
        if allow_execute {
            let router_clone = Arc::clone(router);
            config.handle_call = Some(Arc::new(move |name, args| {
                let r = Arc::clone(&router_clone);
                Box::pin(async move {
                    let (content_items, is_error, error_type) =
                        r.handle_call(&name, &args, None).await;
                    // Convert ContentItem to Value for the explorer
                    let values: Vec<serde_json::Value> = content_items
                        .into_iter()
                        .map(|item| {
                            serde_json::json!({
                                "type": item.content_type,
                                "text": item.data
                            })
                        })
                        .collect();
                    (values, is_error, error_type)
                })
            }));
        }

        config
    }

    /// Synchronously start the MCP server (blocks the current thread).
    ///
    /// Creates a new Tokio runtime and runs the server transport loop.
    /// The transport is determined by `config.transport`:
    /// - `"stdio"` — standard I/O transport
    /// - `"streamable-http"` — HTTP-based streamable transport
    /// - `"sse"` — Server-Sent Events transport
    ///
    /// # Panics
    ///
    /// Panics if called from within an active Tokio runtime (e.g. inside
    /// `#[tokio::main]`).  Use [`async_serve`](Self::async_serve) for
    /// async contexts.
    pub fn serve(&self) -> Result<(), APCoreMCPError> {
        self.serve_with_options(ServeOptions::default())
    }

    /// Synchronously start the MCP server with custom options (blocks the current thread).
    pub fn serve_with_options(&self, opts: ServeOptions) -> Result<(), APCoreMCPError> {
        // Validate explorer prefix if explorer is enabled
        if opts.explorer.explorer && !opts.explorer.explorer_prefix.starts_with('/') {
            return Err(APCoreMCPError::InvalidExplorerPrefix);
        }

        let transport = self.config.transport.to_lowercase();
        if !["stdio", "streamable-http", "sse"].contains(&transport.as_str()) {
            return Err(APCoreMCPError::UnknownTransport(
                self.config.transport.clone(),
            ));
        }

        let (server, router, tools, init_options, version) = self.build_server_components()?;

        tracing::info!(
            "Starting MCP server '{}' v{} with {} tools via {}",
            self.config.name,
            version,
            tools.len(),
            transport,
        );

        let mut transport_manager = TransportManager::new(self.metrics_collector.clone());
        transport_manager.set_module_count(tools.len());
        let transport_manager = Arc::new(transport_manager);

        // Build the McpHandler from the server's registered handlers.
        let handler: Arc<dyn crate::server::transport::McpHandler> = Arc::new(
            ServerHandler::from_server(&server, init_options)
                .ok_or_else(|| APCoreMCPError::ServerError("no tool handlers registered".into()))?,
        );

        // Build explorer router if enabled on HTTP transport.
        let explorer_router = if opts.explorer.explorer && transport != "stdio" {
            let explorer_config = self.build_explorer_config(
                &tools,
                &router,
                &opts.explorer.explorer_prefix,
                opts.explorer.allow_execute,
                &opts.explorer.explorer_title,
                opts.explorer.explorer_project_name.as_deref(),
                opts.explorer.explorer_project_url.as_deref(),
            );
            tracing::info!("Explorer UI mounted at {}", opts.explorer.explorer_prefix);
            Some(create_explorer_mount(explorer_config))
        } else {
            None
        };

        if let Some(ref on_startup) = opts.on_startup {
            on_startup();
        }

        let result = tokio::runtime::Runtime::new()
            .map_err(|e| APCoreMCPError::ServerError(e.to_string()))?
            .block_on(async {
                match transport.as_str() {
                    "streamable-http" => transport_manager
                        .run_streamable_http_with_auth(
                            Arc::clone(&handler),
                            &self.config.host,
                            self.config.port,
                            explorer_router,
                            self.authenticator.clone(),
                            self.config.exempt_paths.clone(),
                        )
                        .await
                        .map_err(|e| APCoreMCPError::ServerError(e.to_string())),
                    #[allow(deprecated)]
                    "sse" => transport_manager
                        .run_sse(
                            Arc::clone(&handler),
                            &self.config.host,
                            self.config.port,
                            explorer_router,
                        )
                        .await
                        .map_err(|e| APCoreMCPError::ServerError(e.to_string())),
                    _ => {
                        // stdio
                        transport_manager
                            .run_stdio(&*handler)
                            .await
                            .map_err(|e| APCoreMCPError::ServerError(e.to_string()))
                    }
                }
            });

        if let Some(ref on_shutdown) = opts.on_shutdown {
            on_shutdown();
        }

        result
    }

    /// Asynchronously build the MCP server and return an axum [`Router`] for embedding.
    ///
    /// This is the async equivalent of [`serve`](Self::serve), but instead of
    /// blocking it returns the built router so callers can mount it in their own
    /// server infrastructure.
    pub async fn async_serve(&self, opts: AsyncServeOptions) -> Result<Router, APCoreMCPError> {
        // Validate explorer prefix
        if opts.explorer.explorer && !opts.explorer.explorer_prefix.starts_with('/') {
            return Err(APCoreMCPError::InvalidExplorerPrefix);
        }

        let (mut server, router, tools, _init_options, version) = self.build_server_components()?;

        tracing::info!(
            "Building MCP app '{}' v{} with {} tools",
            self.config.name,
            version,
            tools.len(),
        );

        // Build the transport manager
        let mut transport_manager = TransportManager::new(self.metrics_collector.clone());
        transport_manager.set_module_count(tools.len());
        let transport_manager = Arc::new(transport_manager);

        // Start the server
        server
            .start()
            .await
            .map_err(|e| APCoreMCPError::ServerError(e.to_string()))?;

        // Build health/metrics router from the transport manager
        let mut app = transport_manager.health_metrics_router();

        // Mount explorer if enabled
        if opts.explorer.explorer {
            let explorer_config = self.build_explorer_config(
                &tools,
                &router,
                &opts.explorer.explorer_prefix,
                opts.explorer.allow_execute,
                &opts.explorer.explorer_title,
                opts.explorer.explorer_project_name.as_deref(),
                opts.explorer.explorer_project_url.as_deref(),
            );
            let explorer_router = create_explorer_mount(explorer_config);
            app = app.merge(explorer_router);
            tracing::info!("Explorer UI mounted at {}", opts.explorer.explorer_prefix);
        }

        Ok(app)
    }

    /// Convert the current registry contents into OpenAI-compatible tool definitions.
    ///
    /// Delegates to [`OpenAIConverter::convert_registry`], passing through the
    /// configured `tags` and `prefix` filters.
    ///
    /// # Arguments
    /// * `embed_annotations` - If true, append annotation hints to descriptions.
    /// * `strict` - If true, enable OpenAI strict mode on schemas.
    pub fn to_openai_tools(
        &self,
        embed_annotations: bool,
        strict: bool,
    ) -> Result<Vec<Value>, APCoreMCPError> {
        let converter = OpenAIConverter::new();

        // Build a JSON registry object from our Registry
        let registry_json = self.build_registry_json();

        let tags_refs: Option<Vec<&str>> = self
            .config
            .tags
            .as_ref()
            .map(|t| t.iter().map(|s| s.as_str()).collect());
        let tags_slice: Option<&[&str]> = tags_refs.as_deref();
        let prefix = self.config.prefix.as_deref();

        let tools = converter.convert_registry(
            &registry_json,
            embed_annotations,
            strict,
            tags_slice,
            prefix,
        )?;

        tracing::debug!("Converted {} tools to OpenAI format", tools.len());
        Ok(tools)
    }

    /// Build a JSON representation of the registry suitable for [`OpenAIConverter`].
    ///
    /// Returns a JSON object `{ "module_id": { "description": "...", "input_schema": {...}, "annotations": {...}, "tags": [...] }, ... }`.
    fn build_registry_json(&self) -> Value {
        let module_ids = self.reg().list(None, None);
        let mut map = serde_json::Map::new();

        for module_id in module_ids {
            if let Some(descriptor) = self.reg().get_definition(module_id) {
                let description = self.reg().describe(module_id);
                let annotations_json =
                    serde_json::to_value(&descriptor.annotations).unwrap_or(Value::Null);
                let tags_json: Vec<Value> = descriptor
                    .tags
                    .iter()
                    .map(|t| Value::String(t.clone()))
                    .collect();

                let entry = serde_json::json!({
                    "description": description,
                    "input_schema": descriptor.input_schema,
                    "output_schema": descriptor.output_schema,
                    "annotations": annotations_json,
                    "tags": tags_json,
                });
                map.insert(module_id.to_string(), entry);
            }
        }

        Value::Object(map)
    }
}

// ---- ExplorerOptions --------------------------------------------------------

/// Shared explorer configuration for [`ServeOptions`] and [`AsyncServeOptions`].
#[derive(Debug, Clone)]
pub struct ExplorerOptions {
    /// Enable the browser-based Tool Explorer UI.
    pub explorer: bool,
    /// URL prefix for the explorer.
    pub explorer_prefix: String,
    /// Page title shown in the explorer browser tab and heading.
    pub explorer_title: String,
    /// Optional project name shown in the explorer footer.
    pub explorer_project_name: Option<String>,
    /// Optional project URL linked in the explorer footer.
    pub explorer_project_url: Option<String>,
    /// Allow tool execution from the explorer UI.
    pub allow_execute: bool,
}

impl Default for ExplorerOptions {
    fn default() -> Self {
        Self {
            explorer: false,
            explorer_prefix: "/explorer".to_string(),
            explorer_title: "MCP Tool Explorer".to_string(),
            explorer_project_name: None,
            explorer_project_url: None,
            allow_execute: false,
        }
    }
}

// ---- ServeOptions -----------------------------------------------------------

/// Options for [`APCoreMCP::serve_with_options`].
#[derive(Default)]
pub struct ServeOptions {
    /// Callback invoked after setup, before the transport starts.
    pub on_startup: Option<Box<dyn Fn() + Send + Sync>>,
    /// Callback invoked after the transport completes (even on error).
    pub on_shutdown: Option<Box<dyn Fn() + Send + Sync>>,
    /// Explorer UI configuration.
    pub explorer: ExplorerOptions,
}

impl std::fmt::Debug for ServeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServeOptions")
            .field("on_startup", &self.on_startup.as_ref().map(|_| "..."))
            .field("on_shutdown", &self.on_shutdown.as_ref().map(|_| "..."))
            .field("explorer", &self.explorer)
            .finish()
    }
}

// ---- AsyncServeOptions ------------------------------------------------------

/// Options for [`APCoreMCP::async_serve`].
#[derive(Debug, Default)]
pub struct AsyncServeOptions {
    /// Explorer UI configuration.
    pub explorer: ExplorerOptions,
}

// ---- APCoreMCPBuilder -------------------------------------------------------

/// Builder for [`APCoreMCP`].
#[derive(Default)]
pub struct APCoreMCPBuilder {
    pub config: APCoreMCPConfig,
    backend: Option<BackendSource>,
    authenticator: Option<Arc<dyn Authenticator>>,
    metrics_collector: Option<Arc<dyn MetricsExporter>>,
    output_formatter: Option<OutputFormatter>,
    approval_handler: Option<Arc<dyn ApprovalHandler>>,
}

impl APCoreMCPBuilder {
    /// Set the backend source (required).
    pub fn backend(mut self, source: impl Into<BackendSource>) -> Self {
        self.backend = Some(source.into());
        self
    }

    /// Set the MCP server name.
    pub fn name(mut self, name: &str) -> Self {
        self.config.name = name.to_string();
        self
    }

    /// Set the MCP server version.
    pub fn version(mut self, version: &str) -> Self {
        self.config.version = Some(version.to_string());
        self
    }

    /// Set the tag filter.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.config.tags = Some(tags);
        self
    }

    /// Set the prefix filter.
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.config.prefix = Some(prefix.to_string());
        self
    }

    /// Set the log level.
    pub fn log_level(mut self, level: &str) -> Self {
        self.config.log_level = Some(level.to_string());
        self
    }

    /// Set the transport type.
    pub fn transport(mut self, transport: &str) -> Self {
        self.config.transport = transport.to_string();
        self
    }

    /// Set the host address.
    pub fn host(mut self, host: &str) -> Self {
        self.config.host = host.to_string();
        self
    }

    /// Set the port number.
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Set whether to validate tool inputs.
    pub fn validate_inputs(mut self, validate: bool) -> Self {
        self.config.validate_inputs = validate;
        self
    }

    /// Set whether authentication is required.
    pub fn require_auth(mut self, require: bool) -> Self {
        self.config.require_auth = require;
        self
    }

    /// Set the authenticator for HTTP transport authentication.
    pub fn authenticator<A: Authenticator + 'static>(mut self, auth: A) -> Self {
        self.authenticator = Some(Arc::new(auth));
        self
    }

    /// Set the metrics collector.
    pub fn metrics_collector(mut self, collector: Arc<dyn MetricsExporter>) -> Self {
        self.metrics_collector = Some(collector);
        self
    }

    /// Set the output formatter.
    ///
    /// **Note:** Reserved — not yet wired into the execution router.
    pub fn output_formatter(mut self, formatter: OutputFormatter) -> Self {
        self.output_formatter = Some(formatter);
        self
    }

    /// Set the approval handler.
    ///
    /// **Note:** Reserved — not yet wired into the executor pipeline.
    pub fn approval_handler(mut self, handler: Arc<dyn ApprovalHandler>) -> Self {
        self.approval_handler = Some(handler);
        self
    }

    /// Set exempt paths for authentication bypass.
    pub fn exempt_paths(mut self, paths: HashSet<String>) -> Self {
        self.config.exempt_paths = Some(paths);
        self
    }

    /// Enable the explorer UI.
    pub fn include_explorer(mut self, include: bool) -> Self {
        self.config.explorer = include;
        self
    }

    /// Set the explorer URL prefix.
    pub fn path_prefix(mut self, prefix: &str) -> Self {
        self.config.explorer_prefix = prefix.to_string();
        self
    }

    /// Set the explorer page title.
    pub fn explorer_title(mut self, title: &str) -> Self {
        self.config.explorer_title = title.to_string();
        self
    }

    /// Set the explorer project name.
    pub fn explorer_project_name(mut self, name: &str) -> Self {
        self.config.explorer_project_name = Some(name.to_string());
        self
    }

    /// Set the explorer project URL.
    pub fn explorer_project_url(mut self, url: &str) -> Self {
        self.config.explorer_project_url = Some(url.to_string());
        self
    }

    /// Set whether tool execution is allowed from the explorer UI.
    pub fn allow_execute(mut self, allow: bool) -> Self {
        self.config.allow_execute = allow;
        self
    }

    /// Consume the builder and produce an [`APCoreMCP`] instance.
    ///
    /// Validates all inputs matching Python validation order, then resolves
    /// the backend into a registry and executor.
    pub fn build(self) -> Result<APCoreMCP, APCoreMCPError> {
        // Validate name
        if self.config.name.is_empty() {
            return Err(APCoreMCPError::EmptyName);
        }
        if self.config.name.len() > 255 {
            return Err(APCoreMCPError::NameTooLong(self.config.name.len()));
        }

        // Validate tags
        if let Some(ref tags) = self.config.tags {
            for tag in tags {
                if tag.is_empty() {
                    return Err(APCoreMCPError::EmptyTag);
                }
            }
        }

        // Validate prefix
        if let Some(ref prefix) = self.config.prefix {
            if prefix.is_empty() {
                return Err(APCoreMCPError::EmptyPrefix);
            }
        }

        // Validate log level
        if let Some(ref level) = self.config.log_level {
            let upper = level.to_uppercase();
            if !VALID_LOG_LEVELS.contains(&upper.as_str()) {
                return Err(APCoreMCPError::InvalidLogLevel(level.clone()));
            }
        }

        // Resolve backend into (registry, executor) pair.
        let backend = self.backend.ok_or_else(|| {
            APCoreMCPError::BackendResolution("backend source is required".to_string())
        })?;

        let (standalone_registry, executor) = match backend {
            BackendSource::ExtensionsDir(path) => {
                return Err(APCoreMCPError::BackendResolution(format!(
                    "ExtensionsDir resolution not yet implemented for path: {}",
                    path.display()
                )));
            }
            BackendSource::Registry(_reg) => {
                // Registry cannot be cloned (contains Box<dyn Module>, callbacks),
                // so we cannot create an Executor from it.  Users must create an
                // Executor themselves and pass it via BackendSource::Executor.
                return Err(APCoreMCPError::BackendResolution(
                    "Registry backend is not supported: Registry cannot be shared with \
                     Executor because it is not Clone. Create an Executor from your \
                     Registry first: `let exec = Arc::new(Executor::new(registry, config)); \
                     builder.backend(exec)`"
                        .to_string(),
                ));
            }
            BackendSource::Executor(exec) => (None, exec),
        };

        Ok(APCoreMCP {
            config: self.config,
            standalone_registry,
            executor,
            authenticator: self.authenticator,
            metrics_collector: self.metrics_collector,
            output_formatter: self.output_formatter,
            approval_handler: self.approval_handler,
        })
    }
}

// ---- Top-level convenience functions ----------------------------------------

/// Configuration for the convenience [`serve`] function.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// MCP server name.
    pub name: String,
    /// MCP server version. `None` defaults to the crate version.
    pub version: Option<String>,
    /// Transport type: "stdio", "streamable-http", or "sse".
    pub transport: String,
    /// Host address for HTTP-based transports.
    pub host: String,
    /// Port number for HTTP-based transports.
    pub port: u16,
    /// Filter modules by tags.
    pub tags: Option<Vec<String>>,
    /// Filter modules by ID prefix.
    pub prefix: Option<String>,
    /// Log level for the apcore_mcp logger.
    pub log_level: Option<String>,
    /// Validate tool inputs against schemas before execution.
    pub validate_inputs: bool,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            name: "apcore-mcp".to_string(),
            version: None,
            transport: "stdio".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            tags: None,
            prefix: None,
            log_level: None,
            validate_inputs: false,
        }
    }
}

/// Configuration for the convenience [`async_serve`] function.
#[derive(Debug, Clone)]
pub struct AsyncServeConfig {
    /// MCP server name.
    pub name: String,
    /// MCP server version. `None` defaults to the crate version.
    pub version: Option<String>,
    /// Filter modules by tags.
    pub tags: Option<Vec<String>>,
    /// Filter modules by ID prefix.
    pub prefix: Option<String>,
    /// Log level for the apcore_mcp logger.
    pub log_level: Option<String>,
    /// Validate tool inputs against schemas before execution.
    pub validate_inputs: bool,
}

impl Default for AsyncServeConfig {
    fn default() -> Self {
        Self {
            name: "apcore-mcp".to_string(),
            version: None,
            tags: None,
            prefix: None,
            log_level: None,
            validate_inputs: false,
        }
    }
}

/// Configuration for the convenience [`to_openai_tools`] function.
#[derive(Debug, Clone, Default)]
pub struct OpenAIToolsConfig {
    /// Embed annotation metadata in tool descriptions.
    pub embed_annotations: bool,
    /// Add `strict: true` for OpenAI Structured Outputs.
    pub strict: bool,
    /// Filter modules by tags.
    pub tags: Option<Vec<String>>,
    /// Filter modules by ID prefix.
    pub prefix: Option<String>,
}

/// Convenience: build and serve in one call (blocking).
///
/// Constructs an [`APCoreMCP`] internally from the given backend source
/// and config, then calls [`APCoreMCP::serve`].
pub fn serve(backend: impl Into<BackendSource>, config: ServeConfig) -> Result<(), APCoreMCPError> {
    let mut builder = APCoreMCP::builder()
        .backend(backend)
        .name(&config.name)
        .transport(&config.transport)
        .host(&config.host)
        .port(config.port)
        .validate_inputs(config.validate_inputs);

    if let Some(version) = config.version.as_deref() {
        builder = builder.version(version);
    }
    if let Some(tags) = config.tags {
        builder = builder.tags(tags);
    }
    if let Some(prefix) = config.prefix.as_deref() {
        builder = builder.prefix(prefix);
    }
    if let Some(log_level) = config.log_level.as_deref() {
        builder = builder.log_level(log_level);
    }

    let mcp = builder.build()?;
    mcp.serve()
}

/// Convenience: build and serve in one call (async).
///
/// Constructs an [`APCoreMCP`] internally from the given backend source
/// and config, then calls [`APCoreMCP::async_serve`].
pub async fn async_serve(
    backend: impl Into<BackendSource>,
    config: AsyncServeConfig,
) -> Result<Router, APCoreMCPError> {
    let mut builder = APCoreMCP::builder()
        .backend(backend)
        .name(&config.name)
        .validate_inputs(config.validate_inputs);

    if let Some(version) = config.version.as_deref() {
        builder = builder.version(version);
    }
    if let Some(tags) = config.tags {
        builder = builder.tags(tags);
    }
    if let Some(prefix) = config.prefix.as_deref() {
        builder = builder.prefix(prefix);
    }
    if let Some(log_level) = config.log_level.as_deref() {
        builder = builder.log_level(log_level);
    }

    let mcp = builder.build()?;
    mcp.async_serve(AsyncServeOptions::default()).await
}

/// Convenience: convert a registry to OpenAI tool definitions without starting a server.
///
/// Constructs an [`APCoreMCP`] internally from the given backend source,
/// then calls [`APCoreMCP::to_openai_tools`].
pub fn to_openai_tools(
    backend: impl Into<BackendSource>,
    config: OpenAIToolsConfig,
) -> Result<Vec<Value>, APCoreMCPError> {
    let mut builder = APCoreMCP::builder().backend(backend);

    if let Some(tags) = config.tags {
        builder = builder.tags(tags);
    }
    if let Some(prefix) = config.prefix.as_deref() {
        builder = builder.prefix(prefix);
    }

    let mcp = builder.build()?;
    mcp.to_openai_tools(config.embed_annotations, config.strict)
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    use apcore::config::Config;
    use apcore::module::{Module, ModuleAnnotations};
    use apcore::registry::ModuleDescriptor;
    use serde_json::json;

    // -- Test helpers ---------------------------------------------------------

    /// Mock module for testing.
    struct MockModule {
        desc: String,
    }

    impl MockModule {
        fn new(desc: &str) -> Self {
            Self {
                desc: desc.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl Module for MockModule {
        fn input_schema(&self) -> Value {
            json!({"type": "object", "properties": {"q": {"type": "string"}}})
        }
        fn output_schema(&self) -> Value {
            json!({})
        }
        fn description(&self) -> &str {
            &self.desc
        }
        async fn execute(
            &self,
            _inputs: Value,
            _ctx: &apcore::context::Context<Value>,
        ) -> Result<Value, apcore::errors::ModuleError> {
            Ok(json!({}))
        }
    }

    /// Create a test registry with the given modules.
    fn make_test_registry(modules: Vec<(&str, &str, Vec<String>)>) -> Registry {
        let mut registry = Registry::new();
        for (name, desc, tags) in modules {
            let module = Box::new(MockModule::new(desc));
            let descriptor = ModuleDescriptor {
                name: name.to_string(),
                annotations: ModuleAnnotations::default(),
                input_schema: json!({"type": "object", "properties": {"q": {"type": "string"}}}),
                output_schema: json!({}),
                enabled: true,
                tags,
                dependencies: vec![],
            };
            registry
                .register_internal(name, module, descriptor)
                .unwrap();
        }
        registry
    }

    /// Create a test APCoreMCP with default config and an empty registry.
    fn make_test_apcore_mcp() -> APCoreMCP {
        let registry = Arc::new(make_test_registry(vec![]));
        let executor = Arc::new(Executor::new(Registry::new(), Config::default()));
        APCoreMCP {
            config: APCoreMCPConfig::default(),
            standalone_registry: Some(registry),
            executor,
            authenticator: None,
            metrics_collector: None,
            output_formatter: None,
            approval_handler: None,
        }
    }

    /// Create a test APCoreMCP with a specific version.
    fn make_test_apcore_mcp_with_version(version: &str) -> APCoreMCP {
        let registry = Arc::new(make_test_registry(vec![]));
        let executor = Arc::new(Executor::new(Registry::new(), Config::default()));
        APCoreMCP {
            config: APCoreMCPConfig {
                version: Some(version.to_string()),
                ..Default::default()
            },
            standalone_registry: Some(registry),
            executor,
            authenticator: None,
            metrics_collector: None,
            output_formatter: None,
            approval_handler: None,
        }
    }

    /// Create a test APCoreMCP with populated registry and tag filter.
    fn make_test_apcore_mcp_with_tags(tags: Vec<String>) -> APCoreMCP {
        let registry = Arc::new(make_test_registry(vec![
            ("mod.public", "Public module", vec!["public".to_string()]),
            ("mod.private", "Private module", vec!["private".to_string()]),
            (
                "mod.both",
                "Both module",
                vec!["public".to_string(), "private".to_string()],
            ),
        ]));
        let executor = Arc::new(Executor::new(Registry::new(), Config::default()));
        APCoreMCP {
            config: APCoreMCPConfig {
                tags: Some(tags),
                ..Default::default()
            },
            standalone_registry: Some(registry),
            executor,
            authenticator: None,
            metrics_collector: None,
            output_formatter: None,
            approval_handler: None,
        }
    }

    /// Create a test APCoreMCP with populated registry and prefix filter.
    fn make_test_apcore_mcp_with_prefix(prefix: &str) -> APCoreMCP {
        let registry = Arc::new(make_test_registry(vec![
            ("my_tool.a", "Tool A", vec![]),
            ("my_tool.b", "Tool B", vec![]),
            ("other.c", "Tool C", vec![]),
        ]));
        let executor = Arc::new(Executor::new(Registry::new(), Config::default()));
        APCoreMCP {
            config: APCoreMCPConfig {
                prefix: Some(prefix.to_string()),
                ..Default::default()
            },
            standalone_registry: Some(registry),
            executor,
            authenticator: None,
            metrics_collector: None,
            output_formatter: None,
            approval_handler: None,
        }
    }

    /// Create a test APCoreMCP with populated registry (no filters).
    fn make_test_apcore_mcp_with_modules() -> APCoreMCP {
        let registry = Arc::new(make_test_registry(vec![
            ("mod.a", "Module A", vec!["api".to_string()]),
            ("mod.b", "Module B", vec!["internal".to_string()]),
        ]));
        let executor = Arc::new(Executor::new(Registry::new(), Config::default()));
        APCoreMCP {
            config: APCoreMCPConfig::default(),
            standalone_registry: Some(registry),
            executor,
            authenticator: None,
            metrics_collector: None,
            output_formatter: None,
            approval_handler: None,
        }
    }

    // -- BackendSource tests --------------------------------------------------

    #[test]
    fn from_string_creates_extensions_dir() {
        let source: BackendSource = "./extensions".into();
        assert!(matches!(source, BackendSource::ExtensionsDir(_)));
    }

    #[test]
    fn from_pathbuf_creates_extensions_dir() {
        let source: BackendSource = PathBuf::from("./extensions").into();
        assert!(matches!(source, BackendSource::ExtensionsDir(_)));
    }

    #[test]
    fn from_str_ref_creates_extensions_dir() {
        let source = BackendSource::from("./my-ext");
        if let BackendSource::ExtensionsDir(p) = source {
            assert_eq!(p, PathBuf::from("./my-ext"));
        } else {
            panic!("expected ExtensionsDir");
        }
    }

    #[test]
    fn from_owned_string_creates_extensions_dir() {
        let source = BackendSource::from(String::from("./exts"));
        if let BackendSource::ExtensionsDir(p) = source {
            assert_eq!(p, PathBuf::from("./exts"));
        } else {
            panic!("expected ExtensionsDir");
        }
    }

    #[test]
    fn from_arc_registry_creates_registry_variant() {
        let reg = Arc::new(Registry::new());
        let source = BackendSource::from(reg.clone());
        assert!(matches!(source, BackendSource::Registry(_)));
    }

    #[test]
    fn from_arc_executor_creates_executor_variant() {
        let reg = Registry::new();
        let exec = Arc::new(Executor::new(reg, Config::default()));
        let source = BackendSource::from(exec.clone());
        assert!(matches!(source, BackendSource::Executor(_)));
    }

    #[test]
    fn backend_source_is_debug() {
        let source: BackendSource = "./ext".into();
        let debug_str = format!("{:?}", source);
        assert!(debug_str.contains("ExtensionsDir"));
    }

    // -- APCoreMCPConfig tests ------------------------------------------------

    #[test]
    fn default_config_has_expected_values() {
        let cfg = APCoreMCPConfig::default();
        assert_eq!(cfg.name, "apcore-mcp");
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8000);
        assert_eq!(cfg.transport, "stdio");
        assert!(!cfg.validate_inputs);
        assert!(cfg.version.is_none());
        assert!(cfg.tags.is_none());
        assert!(cfg.prefix.is_none());
        assert!(cfg.require_auth);
        assert!(!cfg.allow_execute);
    }

    #[test]
    fn config_explorer_defaults() {
        let cfg = APCoreMCPConfig::default();
        assert!(!cfg.explorer);
        assert_eq!(cfg.explorer_prefix, "/explorer");
        assert_eq!(cfg.explorer_title, "APCore MCP Explorer");
        assert_eq!(cfg.explorer_project_name.as_deref(), Some("apcore-mcp"));
        assert_eq!(
            cfg.explorer_project_url.as_deref(),
            Some("https://github.com/aiperceivable/apcore-mcp-rust")
        );
        assert!(!cfg.allow_execute);
    }

    #[test]
    fn config_is_clone() {
        let cfg = APCoreMCPConfig::default();
        let cfg2 = cfg.clone();
        assert_eq!(cfg.name, cfg2.name);
    }

    // -- APCoreMCPError tests -------------------------------------------------

    #[test]
    fn error_display_empty_name() {
        let err = APCoreMCPError::EmptyName;
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn error_display_name_too_long() {
        let err = APCoreMCPError::NameTooLong(300);
        assert!(err.to_string().contains("255"));
    }

    #[test]
    fn error_display_empty_tag() {
        let err = APCoreMCPError::EmptyTag;
        assert!(err.to_string().contains("tag"));
    }

    #[test]
    fn error_display_invalid_log_level() {
        let err = APCoreMCPError::InvalidLogLevel("VERBOSE".into());
        assert!(err.to_string().contains("VERBOSE"));
    }

    #[test]
    fn error_display_empty_prefix() {
        let err = APCoreMCPError::EmptyPrefix;
        assert!(err.to_string().contains("prefix"));
    }

    #[test]
    fn error_display_invalid_explorer_prefix() {
        let err = APCoreMCPError::InvalidExplorerPrefix;
        assert!(err.to_string().contains("explorer_prefix"));
    }

    #[test]
    fn error_display_backend_resolution() {
        let err = APCoreMCPError::BackendResolution("not found".into());
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn error_display_server_error() {
        let err = APCoreMCPError::ServerError("bind failed".into());
        assert!(err.to_string().contains("bind failed"));
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(APCoreMCPError::EmptyName);
        assert!(err.to_string().contains("name"));
    }

    // -- Builder pattern tests (Task 4) ---------------------------------------

    #[test]
    fn builder_requires_backend() {
        let result = APCoreMCP::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_rejects_empty_name() {
        let result = APCoreMCP::builder().backend("./ext").name("").build();
        assert!(matches!(result, Err(APCoreMCPError::EmptyName)));
    }

    #[test]
    fn builder_rejects_name_over_255() {
        let long = "a".repeat(256);
        let result = APCoreMCP::builder().backend("./ext").name(&long).build();
        assert!(matches!(result, Err(APCoreMCPError::NameTooLong(256))));
    }

    #[test]
    fn builder_rejects_empty_tag() {
        let result = APCoreMCP::builder()
            .backend("./ext")
            .tags(vec!["".to_string()])
            .build();
        assert!(matches!(result, Err(APCoreMCPError::EmptyTag)));
    }

    #[test]
    fn builder_rejects_empty_prefix() {
        let result = APCoreMCP::builder().backend("./ext").prefix("").build();
        assert!(matches!(result, Err(APCoreMCPError::EmptyPrefix)));
    }

    #[test]
    fn builder_rejects_invalid_log_level() {
        let result = APCoreMCP::builder()
            .backend("./ext")
            .log_level("VERBOSE")
            .build();
        assert!(matches!(result, Err(APCoreMCPError::InvalidLogLevel(_))));
    }

    #[test]
    fn builder_accepts_valid_log_levels() {
        for level in &["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"] {
            let result = APCoreMCP::builder()
                .backend("./ext")
                .log_level(level)
                .build();
            // May fail on backend resolution, but not on log level validation
            if let Err(e) = &result {
                assert!(!matches!(e, APCoreMCPError::InvalidLogLevel(_)));
            }
        }
    }

    #[test]
    fn builder_sets_all_config_fields() {
        let builder = APCoreMCP::builder()
            .name("test-server")
            .version("1.0.0")
            .tags(vec!["public".into()])
            .prefix("my_")
            .transport("streamable-http")
            .host("0.0.0.0")
            .port(9000)
            .validate_inputs(true)
            .require_auth(false);
        assert_eq!(builder.config.name, "test-server");
        assert_eq!(builder.config.version, Some("1.0.0".to_string()));
        assert_eq!(builder.config.tags, Some(vec!["public".to_string()]));
        assert_eq!(builder.config.prefix, Some("my_".to_string()));
        assert_eq!(builder.config.transport, "streamable-http");
        assert_eq!(builder.config.host, "0.0.0.0");
        assert_eq!(builder.config.port, 9000);
        assert!(builder.config.validate_inputs);
        assert!(!builder.config.require_auth);
    }

    #[test]
    fn builder_with_registry_backend_returns_error() {
        let reg = Arc::new(Registry::new());
        let result = APCoreMCP::builder()
            .backend(BackendSource::Registry(reg))
            .build();
        assert!(result.is_err());
        assert!(matches!(result, Err(APCoreMCPError::BackendResolution(_))));
    }

    #[test]
    fn builder_with_executor_backend_succeeds() {
        let reg = Registry::new();
        let exec = Arc::new(Executor::new(reg, Config::default()));
        let result = APCoreMCP::builder()
            .backend(BackendSource::Executor(exec))
            .build();
        // Executor backend: stored registry is a placeholder (empty),
        // tool discovery uses executor.registry() via reg().
        assert!(result.is_ok());
    }

    // -- Struct and accessor tests (Task 5) -----------------------------------

    #[test]
    fn registry_returns_arc_ref() {
        let mcp = make_test_apcore_mcp();
        let reg = mcp.registry();
        // Should return &Arc<Registry>
        let _ = reg.list(None, None);
    }

    #[test]
    fn executor_returns_arc_ref() {
        let mcp = make_test_apcore_mcp();
        let _exec = mcp.executor();
        // Just verify it returns without panic
    }

    #[test]
    fn tools_returns_module_ids() {
        let mcp = make_test_apcore_mcp_with_modules();
        let tools = mcp.tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains(&"mod.a".to_string()));
        assert!(tools.contains(&"mod.b".to_string()));
    }

    #[test]
    fn tools_returns_empty_for_empty_registry() {
        let mcp = make_test_apcore_mcp();
        let tools = mcp.tools();
        assert!(tools.is_empty());
    }

    #[test]
    fn tools_filters_by_tags() {
        let mcp = make_test_apcore_mcp_with_tags(vec!["public".into()]);
        let tools = mcp.tools();
        // Should include mod.public and mod.both (both have "public" tag)
        assert!(tools.contains(&"mod.public".to_string()));
        assert!(tools.contains(&"mod.both".to_string()));
        assert!(!tools.contains(&"mod.private".to_string()));
    }

    #[test]
    fn tools_filters_by_prefix() {
        let mcp = make_test_apcore_mcp_with_prefix("my_tool.");
        let tools = mcp.tools();
        assert!(tools.contains(&"my_tool.a".to_string()));
        assert!(tools.contains(&"my_tool.b".to_string()));
        assert!(!tools.contains(&"other.c".to_string()));
    }

    // -- build_server_components tests (Task 6) -------------------------------

    #[test]
    fn build_server_components_returns_all_parts() {
        let mcp = make_test_apcore_mcp();
        let components = mcp.build_server_components();
        assert!(components.is_ok());
        let (_server, _router, _tools, _init_options, version) = components.unwrap();
        assert!(!version.is_empty());
    }

    #[test]
    fn build_server_components_uses_custom_version() {
        let mcp = make_test_apcore_mcp_with_version("2.0.0");
        let (_, _, _, _, version) = mcp.build_server_components().unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn build_server_components_defaults_to_crate_version() {
        let mcp = make_test_apcore_mcp();
        let (_, _, _, _, version) = mcp.build_server_components().unwrap();
        assert_eq!(version, crate::VERSION);
    }

    #[test]
    fn build_server_components_applies_tag_filter() {
        let mcp = make_test_apcore_mcp_with_tags(vec!["public".into()]);
        let (_, _, tools, _, _) = mcp.build_server_components().unwrap();
        // Only modules with "public" tag
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"mod.public"));
        assert!(names.contains(&"mod.both"));
        assert!(!names.contains(&"mod.private"));
    }

    #[test]
    fn build_server_components_applies_prefix_filter() {
        let mcp = make_test_apcore_mcp_with_prefix("my_tool.");
        let (_, _, tools, _, _) = mcp.build_server_components().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"my_tool.a"));
        assert!(names.contains(&"my_tool.b"));
        assert!(!names.contains(&"other.c"));
    }

    #[test]
    fn build_server_components_has_init_options() {
        let mcp = make_test_apcore_mcp_with_modules();
        let (_, _, _, init_options, _) = mcp.build_server_components().unwrap();
        assert_eq!(init_options.server_name, "apcore-mcp");
        assert_eq!(init_options.server_version, crate::VERSION);
        // Should have tools capability since we registered handlers
        assert!(init_options.capabilities.tools.is_some());
        // Should have resources capability since we registered resource handlers
        assert!(init_options.capabilities.resources.is_some());
    }

    #[test]
    fn build_server_components_server_has_handlers() {
        let mcp = make_test_apcore_mcp_with_modules();
        let (server, _, _, _, _) = mcp.build_server_components().unwrap();
        assert!(server.has_tool_handlers());
        assert!(server.has_resource_handlers());
    }

    #[test]
    fn build_server_components_tools_match_registry() {
        let mcp = make_test_apcore_mcp_with_modules();
        let (_, _, tools, _, _) = mcp.build_server_components().unwrap();
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let registry_tools = mcp.tools();
        assert_eq!(tools.len(), registry_tools.len());
        for name in &registry_tools {
            assert!(tool_names.contains(&name.as_str()));
        }
    }

    // -- serve-methods tests (Task 7) -----------------------------------------

    fn make_test_apcore_mcp_with_transport(transport: &str) -> APCoreMCP {
        let registry = Arc::new(make_test_registry(vec![]));
        let executor = Arc::new(Executor::new(Registry::new(), Config::default()));
        APCoreMCP {
            config: APCoreMCPConfig {
                transport: transport.to_string(),
                ..Default::default()
            },
            standalone_registry: Some(registry),
            executor,
            authenticator: None,
            metrics_collector: None,
            output_formatter: None,
            approval_handler: None,
        }
    }

    #[test]
    fn serve_rejects_unknown_transport() {
        let mcp = make_test_apcore_mcp_with_transport("websocket");
        let result = mcp.serve();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown transport"));
    }

    #[test]
    fn serve_rejects_invalid_explorer_prefix() {
        let mcp = make_test_apcore_mcp();
        let result = mcp.serve_with_options(ServeOptions {
            explorer: ExplorerOptions {
                explorer: true,
                explorer_prefix: "no-slash".into(),
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(matches!(result, Err(APCoreMCPError::InvalidExplorerPrefix)));
    }

    #[tokio::test]
    async fn async_serve_returns_router() {
        let mcp = make_test_apcore_mcp();
        let result = mcp.async_serve(AsyncServeOptions::default()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn async_serve_rejects_invalid_explorer_prefix() {
        let mcp = make_test_apcore_mcp();
        let result = mcp
            .async_serve(AsyncServeOptions {
                explorer: ExplorerOptions {
                    explorer: true,
                    explorer_prefix: "no-slash".into(),
                    ..Default::default()
                },
            })
            .await;
        assert!(matches!(result, Err(APCoreMCPError::InvalidExplorerPrefix)));
    }

    #[test]
    fn error_display_unknown_transport() {
        let err = APCoreMCPError::UnknownTransport("websocket".into());
        assert!(err.to_string().contains("Unknown transport"));
        assert!(err.to_string().contains("websocket"));
    }

    // -- to-openai-tools tests (Task 8) ---------------------------------------

    #[test]
    fn to_openai_tools_returns_vec_of_values() {
        let mcp = make_test_apcore_mcp();
        let tools = mcp.to_openai_tools(false, false).unwrap();
        assert!(tools.is_empty() || tools.iter().all(|t| t.is_object()));
    }

    #[test]
    fn to_openai_tools_with_modules_returns_tools() {
        let mcp = make_test_apcore_mcp_with_modules();
        let tools = mcp.to_openai_tools(false, false).unwrap();
        assert_eq!(tools.len(), 2);
        for tool in &tools {
            assert!(tool.is_object());
            assert_eq!(tool["type"], "function");
            assert!(tool["function"]["name"].is_string());
            assert!(tool["function"]["description"].is_string());
        }
    }

    #[test]
    fn to_openai_tools_respects_tags() {
        let mcp = make_test_apcore_mcp_with_tags(vec!["public".into()]);
        let tools = mcp.to_openai_tools(false, false).unwrap();
        // Should only include modules with "public" tag (mod.public and mod.both)
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn to_openai_tools_respects_prefix() {
        let mcp = make_test_apcore_mcp_with_prefix("my_tool.");
        let tools = mcp.to_openai_tools(false, false).unwrap();
        // Should only include my_tool.a and my_tool.b
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn to_openai_tools_strict_flag() {
        let mcp = make_test_apcore_mcp_with_modules();
        let tools = mcp.to_openai_tools(false, true).unwrap();
        for tool in &tools {
            assert_eq!(tool["function"]["strict"], true);
        }
    }

    #[test]
    fn to_openai_tools_embed_annotations() {
        let mcp = make_test_apcore_mcp_with_modules();
        let tools_without = mcp.to_openai_tools(false, false).unwrap();
        let tools_with = mcp.to_openai_tools(true, false).unwrap();
        // Both should return the same number of tools
        assert_eq!(tools_without.len(), tools_with.len());
    }

    // -- convenience-functions tests (Task 9) ---------------------------------

    #[test]
    fn convenience_serve_config_has_defaults() {
        let cfg = ServeConfig::default();
        assert_eq!(cfg.transport, "stdio");
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8000);
        assert_eq!(cfg.name, "apcore-mcp");
    }

    #[test]
    fn convenience_async_serve_config_has_defaults() {
        let cfg = AsyncServeConfig::default();
        assert_eq!(cfg.name, "apcore-mcp");
        assert!(cfg.version.is_none());
        assert!(cfg.tags.is_none());
        assert!(cfg.prefix.is_none());
    }

    #[test]
    fn convenience_openai_tools_config_has_defaults() {
        let cfg = OpenAIToolsConfig::default();
        assert!(!cfg.embed_annotations);
        assert!(!cfg.strict);
        assert!(cfg.tags.is_none());
        assert!(cfg.prefix.is_none());
    }

    #[test]
    fn serve_options_default_has_no_callbacks() {
        let opts = ServeOptions::default();
        assert!(opts.on_startup.is_none());
        assert!(opts.on_shutdown.is_none());
        assert!(!opts.explorer.explorer);
    }

    #[test]
    fn async_serve_options_default() {
        let opts = AsyncServeOptions::default();
        assert!(!opts.explorer.explorer);
        assert_eq!(opts.explorer.explorer_prefix, "/explorer");
    }

    // -- build_registry_json tests (Task 8, helper) ---------------------------

    #[test]
    fn build_registry_json_empty_registry() {
        let mcp = make_test_apcore_mcp();
        let json = mcp.build_registry_json();
        assert!(json.is_object());
        assert_eq!(json.as_object().unwrap().len(), 0);
    }

    #[test]
    fn build_registry_json_with_modules() {
        let mcp = make_test_apcore_mcp_with_modules();
        let json = mcp.build_registry_json();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("mod.a"));
        assert!(obj.contains_key("mod.b"));

        let mod_a = &obj["mod.a"];
        assert!(mod_a["description"].is_string());
        assert!(mod_a["input_schema"].is_object());
        assert!(mod_a["tags"].is_array());
    }
}
