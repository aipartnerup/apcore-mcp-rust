//! Explorer sub-module — introspection mount for browsing registered tools.
//!
//! Delegates to `mcp-embedded-ui` for HTML rendering and HTTP handlers.

mod mount;

pub use mount::{create_explorer_mount, CallResult, ExplorerConfig, HandleCallFn, ToolInfo};
