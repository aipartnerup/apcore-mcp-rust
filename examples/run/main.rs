//! Launch MCP server with example extension modules.
//!
//! Usage (from the project root):
//!
//!     cargo run --example run
//!
//! Then open http://127.0.0.1:8000/explorer in your browser.
//!
//! Available tools:
//!   - text.echo     — echo text back, optionally uppercase
//!   - math.calc     — basic arithmetic (add, sub, mul, div)
//!   - greeting      — personalized greeting in different styles

mod modules;

use std::sync::Arc;

use apcore::config::Config;
use apcore::executor::Executor;
use apcore::module::{Module, ModuleAnnotations};
use apcore::registry::registry::{ModuleDescriptor, Registry};
use apcore_mcp::{APCoreMCP, ServeOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (respects RUST_LOG env var)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 1. Create a registry and register example modules.
    let mut registry = Registry::new();

    let text_echo = modules::TextEcho;
    registry.register(
        "text.echo",
        Box::new(text_echo),
        ModuleDescriptor {
            name: "text.echo".into(),
            annotations: ModuleAnnotations {
                readonly: true,
                idempotent: true,
                open_world: false,
                ..Default::default()
            },
            input_schema: modules::TextEcho.input_schema(),
            output_schema: modules::TextEcho.output_schema(),
            tags: vec!["text".into(), "utility".into()],
            enabled: true,
            dependencies: vec![],
        },
    )?;

    let math_calc = modules::MathCalc;
    registry.register(
        "math.calc",
        Box::new(math_calc),
        ModuleDescriptor {
            name: "math.calc".into(),
            annotations: ModuleAnnotations {
                readonly: true,
                idempotent: true,
                open_world: false,
                ..Default::default()
            },
            input_schema: modules::MathCalc.input_schema(),
            output_schema: modules::MathCalc.output_schema(),
            tags: vec!["math".into(), "utility".into()],
            enabled: true,
            dependencies: vec![],
        },
    )?;

    let greeting = modules::Greeting;
    registry.register(
        "greeting",
        Box::new(greeting),
        ModuleDescriptor {
            name: "greeting".into(),
            annotations: ModuleAnnotations {
                readonly: true,
                open_world: false,
                ..Default::default()
            },
            input_schema: modules::Greeting.input_schema(),
            output_schema: modules::Greeting.output_schema(),
            tags: vec!["text".into(), "fun".into()],
            enabled: true,
            dependencies: vec![],
        },
    )?;

    tracing::info!("Registered {} modules", registry.list(None, None).len());

    // 2. Create executor from registry.
    let executor = Arc::new(Executor::new(registry, Config::default()));

    // 3. Build the APCoreMCP bridge with explorer enabled.
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
    tracing::info!("Explorer UI:      http://127.0.0.1:8000/explorer");

    // 4. Serve (blocks the current thread).
    mcp.serve_with_options(ServeOptions {
        explorer: true,
        allow_execute: true,
        explorer_prefix: "/explorer".to_string(),
        explorer_title: "APCore MCP Examples Explorer".to_string(),
        explorer_project_name: Some("apcore-mcp".to_string()),
        explorer_project_url: Some("https://github.com/aiperceivable/apcore-mcp-rust".to_string()),
        on_startup: Some(Box::new(|| {
            println!("\n  MCP server ready at http://127.0.0.1:8000");
            println!("  Explorer UI at     http://127.0.0.1:8000/explorer\n");
        })),
        on_shutdown: Some(Box::new(|| {
            println!("MCP server shut down.");
        })),
    })?;

    Ok(())
}
