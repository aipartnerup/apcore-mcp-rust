//! MCP error formatter — naming-parity facade over [`crate::adapters::errors`].
//!
//! Python exposes the MCP error formatter as `formatter.py`; TypeScript as
//! `mcp-error-formatter.ts`. Rust's canonical implementation lives in
//! `adapters::errors` ([`McpErrorFormatter`], [`register_mcp_formatter`])
//! for historical reasons. This module re-exports those symbols under the
//! `formatter` name so cross-language readers can find the same surface
//! at the expected path. [D8-003]
//!
//! No implementation lives here — see [`crate::adapters::errors`] for the
//! actual formatter logic and tests.

pub use crate::adapters::errors::{register_mcp_formatter, McpErrorFormatter};
