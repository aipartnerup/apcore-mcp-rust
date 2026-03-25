//! Launch MCP server with example extension modules.
//!
//! Usage (from the project root):
//!
//!     cargo run --example run
//!
//! Then open http://127.0.0.1:8000/explorer/ in your browser.

use std::sync::Arc;

use apcore::config::Config;
use apcore::executor::Executor;
use apcore::registry::registry::Registry;
use apcore_mcp::{APCoreMCP, ServeOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (respects RUST_LOG env var)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 1. Create a registry and executor.
    //
    //    In a real application, you would register your own modules or use
    //    a discoverer to find them. For this example we start with an empty
    //    registry — the Explorer UI will show zero tools but still render.
    let registry = Registry::new();
    let executor = Arc::new(Executor::new(registry, Config::default()));
    tracing::info!("Executor created (register modules to see tools in Explorer)");

    // 2. Build the APCoreMCP bridge with explorer enabled.
    let mcp = APCoreMCP::builder()
        .backend(executor)
        .name("apcore-mcp-examples")
        .transport("streamable-http")
        .host("127.0.0.1")
        .port(8000)
        .include_explorer(true)
        .allow_execute(true)
        .validate_inputs(true)
        .require_auth(false)
        .explorer_title("APCore MCP Examples Explorer")
        .explorer_project_name("apcore-mcp")
        .explorer_project_url("https://github.com/aiperceivable/apcore-mcp-rust")
        .build()?;

    tracing::info!("Registered tools: {:?}", mcp.tools());
    tracing::info!("Explorer UI:      http://127.0.0.1:8000/explorer/");

    // 3. Serve (blocks the current thread).
    mcp.serve_with_options(ServeOptions {
        explorer: true,
        allow_execute: true,
        explorer_prefix: "/explorer".to_string(),
        explorer_title: "APCore MCP Examples Explorer".to_string(),
        explorer_project_name: Some("apcore-mcp".to_string()),
        explorer_project_url: Some("https://github.com/aiperceivable/apcore-mcp-rust".to_string()),
        on_startup: Some(Box::new(|| {
            println!("\n  MCP server ready at http://127.0.0.1:8000");
            println!("  Explorer UI at     http://127.0.0.1:8000/explorer/\n");
        })),
        on_shutdown: Some(Box::new(|| {
            println!("MCP server shut down.");
        })),
    })?;

    Ok(())
}
