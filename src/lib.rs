//! # apcore-mcp
//!
//! MCP bridge for apcore — expose apcore modules as MCP tools.
//!
//! This crate provides the bridge layer that translates apcore module
//! registries and executors into MCP-compatible tool servers.

pub mod server;
pub mod auth;
pub mod adapters;
pub mod converters;
pub mod explorer;
pub mod constants;
pub mod helpers;
pub mod apcore_mcp;
pub mod cli;
mod utils;

/// Crate version, kept in sync with Cargo.toml.
pub const VERSION: &str = "0.10.0";

// ---- Re-exports: core bridge ------------------------------------------------
pub use crate::apcore_mcp::{APCoreMCP, APCoreMCPBuilder, APCoreMCPConfig, APCoreMCPError, BackendSource};
pub use crate::apcore_mcp::{ServeOptions, AsyncServeOptions};
pub use crate::apcore_mcp::{ServeConfig, AsyncServeConfig, OpenAIToolsConfig};

// ---- Re-exports: top-level convenience functions ----------------------------
pub use crate::apcore_mcp::{serve, async_serve, to_openai_tools};

// ---- Re-exports: auth -------------------------------------------------------
pub use crate::auth::protocol::Authenticator;
pub use crate::auth::jwt::{JWTAuthenticator, ClaimMapping};
pub use crate::auth::middleware::{AuthMiddlewareLayer, AuthMiddlewareService, AUTH_IDENTITY, extract_headers};

// ---- Re-exports: server -----------------------------------------------------
pub use crate::server::server::{MCPServer, MCPServerConfig, TransportKind, RegistryOrExecutor};
pub use crate::server::factory::MCPServerFactory;
pub use crate::server::router::ExecutionRouter;
pub use crate::server::transport::TransportManager;
pub use crate::server::listener::RegistryListener;

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
pub use crate::helpers::{report_progress, elicit, ElicitResult};
