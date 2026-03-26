//! Launch MCP server with example extension modules.
//!
//! Usage (from the project root):
//!
//!     cargo run --example run
//!
//! Enable authentication (plain bearer token):
//!
//!     AUTH_TOKEN=my-secret cargo run --example run
//!
//! Then open http://127.0.0.1:8000/explorer in your browser.
//!
//! Test with curl:
//!
//!     curl http://localhost:8000/health                             # 200 (exempt)
//!     curl -X POST http://localhost:8000/mcp ...                    # 401 (no token)
//!     curl -H "Authorization: Bearer my-secret" -X POST ...        # 200
//!
//! Available tools:
//!   - text.echo     — echo text back, optionally uppercase
//!   - math.calc     — basic arithmetic (add, sub, mul, div)
//!   - greeting      — personalized greeting in different styles

mod modules;

use std::collections::HashMap;
use std::sync::Arc;

use apcore::config::Config;
use apcore::executor::Executor;
use apcore::module::{Module, ModuleAnnotations};
use apcore::registry::registry::{ModuleDescriptor, Registry};
use apcore_mcp::auth::protocol::{Authenticator, Identity};
use apcore_mcp::{APCoreMCP, ExplorerOptions, ServeOptions};
use async_trait::async_trait;

/// Simple bearer token authenticator — checks Authorization header
/// against a fixed secret. For demo purposes only.
struct BearerTokenAuth {
    token: String,
}

#[async_trait]
impl Authenticator for BearerTokenAuth {
    async fn authenticate(&self, headers: &HashMap<String, String>) -> Option<Identity> {
        let auth = headers.get("authorization")?;
        let bearer = auth.strip_prefix("Bearer ")?;
        if bearer == self.token {
            Some(Identity {
                id: "demo-user".into(),
                identity_type: "bearer".into(),
                roles: vec!["admin".into()],
                attrs: Default::default(),
            })
        } else {
            None
        }
    }
}

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

    registry.register(
        "text.echo",
        Box::new(modules::TextEcho),
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

    registry.register(
        "math.calc",
        Box::new(modules::MathCalc),
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

    registry.register(
        "greeting",
        Box::new(modules::Greeting),
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

    // 3. Build APCoreMCP with optional auth.
    let auth_token = std::env::var("AUTH_TOKEN").ok();
    let mut builder = APCoreMCP::builder()
        .backend(executor)
        .name("apcore-mcp-examples")
        .transport("streamable-http")
        .host("127.0.0.1")
        .port(8000)
        .include_explorer(true)
        .allow_execute(true)
        .validate_inputs(true)
        .explorer_title("APCore MCP Examples Explorer")
        .explorer_project_name("apcore-mcp")
        .explorer_project_url("https://github.com/aiperceivable/apcore-mcp-rust");

    if let Some(ref token) = auth_token {
        let auth = BearerTokenAuth {
            token: token.clone(),
        };
        builder = builder.authenticator(auth).require_auth(true);
        tracing::info!("Authentication:  enabled (Bearer token)");
        tracing::info!("Token:           {token}");
        tracing::info!("Usage:           curl -H \"Authorization: Bearer {token}\" ...");
    } else {
        builder = builder.require_auth(false);
        tracing::info!("Authentication:  disabled (set AUTH_TOKEN to enable)");
    }

    let mcp = builder.build()?;

    tracing::info!("Registered tools: {:?}", mcp.tools());
    tracing::info!("Explorer UI:      http://127.0.0.1:8000/explorer");

    // 4. Serve (blocks the current thread).
    mcp.serve_with_options(ServeOptions {
        explorer: ExplorerOptions {
            explorer: true,
            allow_execute: true,
            explorer_prefix: "/explorer".to_string(),
            explorer_title: "APCore MCP Examples Explorer".to_string(),
            explorer_project_name: Some("apcore-mcp".to_string()),
            explorer_project_url: Some(
                "https://github.com/aiperceivable/apcore-mcp-rust".to_string(),
            ),
        },
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
