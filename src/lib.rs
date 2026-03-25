//! # apcore-mcp
//!
//! MCP bridge for apcore — expose apcore modules as MCP tools.
//!
//! This crate provides the bridge layer that translates apcore module
//! registries and executors into MCP-compatible tool servers.

pub mod adapters;
pub mod apcore_mcp;
pub mod auth;
pub mod cli;
pub mod constants;
pub mod converters;
pub mod explorer;
pub mod helpers;
pub mod server;
mod utils;

/// Crate version, kept in sync with Cargo.toml.
pub const VERSION: &str = "0.11.0";

// ---- Re-exports: core bridge ------------------------------------------------
pub use crate::apcore_mcp::{
    APCoreMCP, APCoreMCPBuilder, APCoreMCPConfig, APCoreMCPError, BackendSource,
};
pub use crate::apcore_mcp::{AsyncServeConfig, OpenAIToolsConfig, ServeConfig};
pub use crate::apcore_mcp::{AsyncServeOptions, ServeOptions};

// ---- Re-exports: top-level convenience functions ----------------------------
pub use crate::apcore_mcp::{async_serve, serve, to_openai_tools};

// ---- Re-exports: auth -------------------------------------------------------
pub use crate::auth::jwt::{ClaimMapping, JWTAuthenticator};
pub use crate::auth::middleware::{
    extract_headers, AuthMiddlewareLayer, AuthMiddlewareService, AUTH_IDENTITY,
};
pub use crate::auth::protocol::Authenticator;

// ---- Re-exports: server -----------------------------------------------------
pub use crate::server::factory::MCPServerFactory;
pub use crate::server::listener::RegistryListener;
pub use crate::server::router::ExecutionRouter;
pub use crate::server::server::{MCPServer, MCPServerConfig, RegistryOrExecutor, TransportKind};
pub use crate::server::transport::TransportManager;

// ---- Re-exports: adapters ---------------------------------------------------
pub use crate::adapters::AdapterError;
pub use crate::adapters::AnnotationMapper;
pub use crate::adapters::ElicitationApprovalHandler;
pub use crate::adapters::ErrorMapper;
pub use crate::adapters::ModuleIDNormalizer;
pub use crate::adapters::SchemaConverter;

// ---- Re-exports: converters -------------------------------------------------
pub use crate::converters::openai::{ConverterError, OpenAIConverter};

// ---- Re-exports: helpers ----------------------------------------------------
pub use crate::helpers::{elicit, report_progress, ElicitResult};
