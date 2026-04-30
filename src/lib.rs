//! # apcore-mcp
//!
//! MCP bridge for apcore — expose apcore modules as MCP tools.
//!
//! This crate provides the bridge layer that translates apcore module
//! registries and executors into MCP-compatible tool servers.

pub mod acl_builder;
pub mod adapters;
pub mod apcore_mcp;
pub mod auth;
pub mod cli;
pub mod config;
pub mod constants;
pub mod converters;
pub mod explorer;
pub mod helpers;
/// Inspector sub-module — placeholder for future port from Python inspector/.
/// [D8-004]
pub mod inspector;
pub mod middleware_builder;
pub mod server;
/// Crate version, kept in sync with Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---- Re-exports: core bridge ------------------------------------------------
pub use crate::apcore_mcp::{
    APCoreMCP, APCoreMCPBuilder, APCoreMCPConfig, APCoreMCPError, BackendSource,
};
pub use crate::apcore_mcp::{AsyncServeConfig, OpenAIToolsConfig, ServeConfig};
pub use crate::apcore_mcp::{AsyncServeOptions, ExplorerOptions, ServeOptions};

// ---- Re-exports: top-level convenience functions ----------------------------
pub use crate::apcore_mcp::{async_serve, serve, to_openai_tools};

// ---- Re-exports: auth -------------------------------------------------------
pub use crate::auth::jwt::{ClaimMapping, JWTAuthenticator};
// [D9-009] extract_headers is internal (pub(crate)) — not re-exported.
pub use crate::auth::middleware::{AuthMiddlewareLayer, AuthMiddlewareService, AUTH_IDENTITY};
pub use crate::auth::protocol::Authenticator;

// ---- Re-exports: server -----------------------------------------------------
pub use crate::server::factory::MCPServerFactory;
pub use crate::server::listener::RegistryListener;
pub use crate::server::router::ExecutionRouter;
pub use crate::server::server::{MCPServer, MCPServerConfig, RegistryOrExecutor, TransportKind};
pub use crate::server::transport::TransportManager;

// ---- Re-exports: adapters ---------------------------------------------------
pub use crate::adapters::register_mcp_formatter;
pub use crate::adapters::AdapterError;
pub use crate::adapters::AnnotationMapper;
pub use crate::adapters::ElicitationApprovalHandler;
pub use crate::adapters::ErrorMapper;
pub use crate::adapters::McpErrorFormatter;
pub use crate::adapters::ModuleIDNormalizer;
pub use crate::adapters::SchemaConverter;

// ---- Re-exports: config bus -------------------------------------------------
pub use crate::config::{mcp_defaults, register_mcp_namespace, MCP_ENV_PREFIX, MCP_NAMESPACE};

// ---- Re-exports: converters -------------------------------------------------
pub use crate::converters::openai::{ConverterError, OpenAIConverter};

// ---- Re-exports: helpers ----------------------------------------------------
pub use crate::helpers::{elicit, report_progress, ElicitResult};
