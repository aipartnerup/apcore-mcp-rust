//! Server sub-module — MCP server, factory, router, transport, and listener.

pub mod async_task_bridge;
pub mod factory;
pub mod listener;
pub mod router;
#[allow(clippy::module_inception)]
pub mod server;
pub mod transport;
pub mod types;

// ---- Re-exports: key public types ------------------------------------------
pub use self::listener::RegistryListener;
pub use self::server::{MCPServer, MCPServerConfig, RegistryOrExecutor, TransportKind};
