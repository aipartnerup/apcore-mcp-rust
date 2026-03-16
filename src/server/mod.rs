//! Server sub-module — MCP server, factory, router, transport, and listener.

pub mod factory;
pub mod router;
pub mod transport;
pub mod listener;
pub mod server;
pub mod types;

// ---- Re-exports: key public types ------------------------------------------
pub use self::server::{MCPServer, MCPServerConfig, TransportKind, RegistryOrExecutor};
pub use self::listener::RegistryListener;
