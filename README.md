<div align="center">
  <img src="https://raw.githubusercontent.com/aiperceivable/apcore-mcp/main/apcore-mcp-logo.svg" alt="apcore-mcp logo" width="200"/>
</div>

# apcore-mcp-rust

Automatic MCP Server & OpenAI Tools Bridge for apcore (Rust edition).

**apcore-mcp** turns any [apcore](https://github.com/aiperceivable/apcore)-based project into an MCP Server and OpenAI tool provider — with **zero code changes** to your existing project.

```
┌──────────────────┐
│  axum-apcore     │  ← your existing apcore project (unchanged)
│  project         │
└────────┬─────────┘
         │  extensions directory
         ▼
┌──────────────────┐
│  apcore-mcp-rust │  ← just install & point to extensions dir
└───┬──────────┬───┘
    │          │
    ▼          ▼
  MCP       OpenAI
 Server      Tools
```

## Design Philosophy

- **Zero intrusion** — your apcore project needs no code changes, no imports, no dependencies on apcore-mcp
- **Zero configuration** — point to an extensions directory, everything is auto-discovered
- **Pure adapter** — apcore-mcp reads from the apcore Registry; it never modifies your modules
- **Works with any apcore project** — if it uses the apcore Module Registry, apcore-mcp can serve it

## Documentation

For full documentation, including Quick Start guides, visit:
**[https://aiperceivable.github.io/apcore-mcp/](https://aiperceivable.github.io/apcore-mcp/)**

## Installation

### As a library

```sh
cargo add apcore-mcp
```

### As a CLI tool

```sh
cargo install apcore-mcp
```

Requires Rust 1.75+ and `apcore >= 0.13.0`.

## Quick Start

### Zero-code approach (CLI)

If you already have an apcore-based project with an extensions directory, just run:

```sh
apcore-mcp --extensions-dir /path/to/your/extensions
```

All modules are auto-discovered and exposed as MCP tools. No code needed.

### Programmatic approach (Rust API)

The `APCoreMCP` builder is the recommended entry point — one object, all capabilities:

```rust
use apcore_mcp::APCoreMCP;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mcp = APCoreMCP::builder()
        .backend("./extensions")
        .name("my-server")
        .transport("streamable-http")
        .port(8000)
        .build()?;

    // Launch as MCP Server
    mcp.serve()?;

    Ok(())
}
```

You can also pass an existing `Registry` or `Executor`:

```rust
use std::sync::Arc;
use apcore::registry::registry::Registry;
use apcore_mcp::APCoreMCP;

let registry = Arc::new(Registry::new());
// ... register modules ...

let mcp = APCoreMCP::builder()
    .backend(registry)
    .name("my-server")
    .tags(vec!["public".into()])
    .build()?;
```

<details>
<summary>Function-based API (still supported)</summary>

```rust
use apcore_mcp::{serve, to_openai_tools, ServeConfig, OpenAIToolsConfig};

serve("./extensions", ServeConfig::default())?;

let tools = to_openai_tools("./extensions", OpenAIToolsConfig::default())?;
```
</details>

## Integration with Existing Projects

### Typical apcore project structure

```
your-project/
├── extensions/          ← modules live here
│   ├── image_resize/
│   ├── text_translate/
│   └── ...
├── src/main.rs          ← your existing code (untouched)
└── ...
```

### Adding MCP support

No changes to your project. Just run apcore-mcp alongside it:

```sh
# Install (one time)
cargo install apcore-mcp

# Run
apcore-mcp --extensions-dir ./extensions
```

Your existing application continues to work exactly as before. apcore-mcp operates as a separate process that reads from the same extensions directory.

### Adding OpenAI tools support

For OpenAI integration, a thin script is needed — but still **no changes to your existing modules**:

```rust
use apcore_mcp::{to_openai_tools, OpenAIToolsConfig};

let tools = to_openai_tools("./extensions", OpenAIToolsConfig {
    strict: true,
    ..Default::default()
})?;
// Use with the OpenAI API
```

## MCP Client Configuration

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "apcore": {
      "command": "apcore-mcp",
      "args": ["--extensions-dir", "/path/to/your/extensions"]
    }
  }
}
```

### Claude Code

Add to `.mcp.json` in your project root:

```json
{
  "mcpServers": {
    "apcore": {
      "command": "apcore-mcp",
      "args": ["--extensions-dir", "./extensions"]
    }
  }
}
```

### Cursor

Add to `.cursor/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "apcore": {
      "command": "apcore-mcp",
      "args": ["--extensions-dir", "./extensions"]
    }
  }
}
```

### Remote HTTP access

```sh
apcore-mcp --extensions-dir ./extensions \
    --transport streamable-http \
    --host 0.0.0.0 \
    --port 9000
```

Connect any MCP client to `http://your-host:9000/mcp`.

## CLI Reference

```
apcore-mcp --extensions-dir PATH [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--extensions-dir` | *(required)* | Path to apcore extensions directory |
| `--transport` | `stdio` | Transport: `stdio`, `streamable-http`, or `sse` |
| `--host` | `127.0.0.1` | Host for HTTP-based transports |
| `--port` | `8000` | Port for HTTP-based transports (1-65535) |
| `--name` | `apcore-mcp` | MCP server name (max 255 chars) |
| `--version` | package version | MCP server version string |
| `--log-level` | `INFO` | Logging: `DEBUG`, `INFO`, `WARNING`, `ERROR` |
| `--explorer` | off | Enable the browser-based Tool Explorer UI (HTTP only) |
| `--explorer-prefix` | `/explorer` | URL prefix for the explorer UI |
| `--allow-execute` | off | Allow tool execution from the explorer UI |
| `--jwt-secret` | — | JWT secret key for Bearer token auth (HTTP only) |
| `--jwt-key-file` | — | Path to PEM key file for JWT verification (e.g. RS256 public key) |
| `--jwt-algorithm` | `HS256` | JWT signing algorithm |
| `--jwt-audience` | — | Expected JWT audience claim |
| `--jwt-issuer` | — | Expected JWT issuer claim |
| `--jwt-require-auth` | on | Require valid token; use `--no-jwt-require-auth` for permissive mode |
| `--exempt-paths` | — | Comma-separated paths exempt from auth (e.g. `/health,/metrics`) |
| `--approval` | `off` | Approval handler: `elicit`, `auto-approve`, `always-deny`, or `off` |

JWT key resolution priority: `--jwt-key-file` > `--jwt-secret` > `JWT_SECRET` environment variable.

Exit codes: `0` normal, `1` invalid arguments, `2` startup failure.

## Rust API Reference

### `APCoreMCP` (recommended)

The unified entry point — configure once, use everywhere:

```rust
use apcore_mcp::{APCoreMCP, ServeOptions};

let mcp = APCoreMCP::builder()
    .backend("./extensions")             // path, Arc<Registry>, or Arc<Executor>
    .name("apcore-mcp")                  // server name
    .version("1.0.0")                    // defaults to crate version
    .tags(vec!["public".into()])         // filter modules by tags
    .prefix("image")                     // filter modules by ID prefix
    .transport("streamable-http")        // "stdio" | "streamable-http" | "sse"
    .host("127.0.0.1")                  // host for HTTP transports
    .port(8000)                          // port for HTTP transports
    .validate_inputs(true)               // validate inputs against schemas
    .authenticator(auth)                 // Authenticator for JWT/token auth (HTTP only)
    .metrics_collector(collector)         // MetricsExporter for /metrics endpoint
    .output_formatter(formatter)         // custom result formatting
    .approval_handler(handler)           // approval handler for runtime approval
    .build()?;

// Launch as MCP server (blocking)
mcp.serve(ServeOptions::default())?;

// Export as OpenAI tools
let tools = mcp.to_openai_tools(false, true)?;

// Inspect
let tool_names = mcp.tools();     // list of module IDs
let registry = mcp.registry();    // underlying Registry
let executor = mcp.executor();    // underlying Executor
```

### `serve()` (function-based)

```rust
use apcore_mcp::{serve, ServeConfig};

serve("./extensions", ServeConfig {
    transport: "streamable-http".into(),
    host: "127.0.0.1".into(),
    port: 8000,
    name: "apcore-mcp".into(),
    explorer: true,
    allow_execute: true,
    ..Default::default()
})?;
```

### `async_serve()`

Embed the MCP server into a larger application (e.g. co-host with other services):

```rust
use apcore_mcp::AsyncServeOptions;

let app = mcp.async_serve(AsyncServeOptions {
    explorer: true,
    ..Default::default()
}).await?;
// Mount `app` into your own axum Router
```

### Tool Explorer

When `explorer=true` is passed to `serve()`, a browser-based Tool Explorer UI is mounted on HTTP transports. It provides an interactive page for browsing tool schemas and testing tool execution.

```rust
mcp.serve(ServeOptions {
    explorer: true,
    allow_execute: true,
    ..Default::default()
})?;
// Open http://127.0.0.1:8000/explorer/ in a browser
```

**Endpoints:**

| Endpoint | Description |
|----------|-------------|
| `GET /explorer/` | Interactive HTML page (self-contained, no external dependencies) |
| `GET /explorer/tools` | JSON array of all tools with name, description, annotations |
| `GET /explorer/tools/<name>` | Full tool detail with inputSchema |
| `POST /explorer/tools/<name>/call` | Execute a tool (requires `allow_execute=true`) |

- **HTTP transports only** (`streamable-http`, `sse`). Silently ignored for `stdio`.
- **Execution disabled by default** — set `allow_execute=true` to enable Try-it.
- **Custom prefix** — use `explorer_prefix="/browse"` to mount at a different path.

### JWT Authentication

Optional Bearer token authentication for HTTP transports. Supports symmetric (HS256) and asymmetric (RS256) algorithms.

```rust
use apcore_mcp::JWTAuthenticator;

let auth = JWTAuthenticator::new("my-secret", None, None, None, None, None, None);

let mcp = APCoreMCP::builder()
    .backend("./extensions")
    .transport("streamable-http")
    .authenticator(auth)
    .build()?;
```

**Permissive mode** — allow unauthenticated access (identity is `None` when no token is provided):

```rust
let auth = JWTAuthenticator::new("my-secret", None, None, None, None, None, Some(false));
```

**Path exemption** — bypass auth for specific paths via CLI:

```sh
apcore-mcp --extensions-dir ./extensions --jwt-secret my-secret --exempt-paths /health,/metrics
```

### Approval Mechanism

Optional runtime approval for tool execution. Bridges MCP elicitation to apcore's approval system.

```rust
use apcore_mcp::ElicitationApprovalHandler;

let handler = ElicitationApprovalHandler::new(None);

let mcp = APCoreMCP::builder()
    .backend("./extensions")
    .approval_handler(Arc::new(handler))
    .build()?;
```

**Built-in handlers:**

| Handler | Description |
|---------|-------------|
| `ElicitationApprovalHandler` | Prompts the MCP client for user confirmation via elicitation |
| `AutoApproveHandler` | Auto-approves all requests (dev/testing only) |
| `AlwaysDenyHandler` | Rejects all requests (enforcement) |

CLI usage:

```sh
apcore-mcp --extensions-dir ./extensions --approval elicit
```

### Output Formatting

By default, tool execution results are serialized as JSON. You can customize this by passing an `output_formatter` closure that converts a `serde_json::Value` into a string.

```rust
use apcore_mcp::APCoreMCP;

let formatter = Box::new(|val: &serde_json::Value| -> Result<String, Box<dyn std::error::Error>> {
    Ok(serde_json::to_string_pretty(val)?)
});

let mcp = APCoreMCP::builder()
    .backend("./extensions")
    .output_formatter(formatter)
    .build()?;
```

The `output_formatter` is also available on `ExecutionRouter` directly.

### Extension Helpers

Modules can report progress and request user input during execution via MCP protocol callbacks. Both helpers no-op gracefully when called outside an MCP context.

```rust
use apcore_mcp::{report_progress, elicit};

// Inside a module's execute():
report_progress(&context, progress_cb.as_ref(), 50.0, Some(100.0), Some("Halfway done")).await;

let result = elicit(&context, elicit_cb.as_ref(), "Confirm deletion?", Some(&schema)).await;
if let Some(r) = result {
    if r.action == ElicitAction::Accept {
        // proceed
    }
}
```

### `/metrics` Prometheus Endpoint

When `metrics_collector` is provided, a `/metrics` HTTP endpoint is exposed that returns metrics in Prometheus text exposition format.

- **Available on HTTP-based transports only** (`streamable-http`, `sse`). Not available with `stdio` transport.
- **Returns Prometheus text format** with Content-Type `text/plain; version=0.0.4; charset=utf-8`.
- **Returns 404** when no `metrics_collector` is configured.

### `to_openai_tools()`

```rust
use apcore_mcp::{to_openai_tools, OpenAIToolsConfig};

let tools = to_openai_tools("./extensions", OpenAIToolsConfig {
    embed_annotations: false,   // append annotation hints to descriptions
    strict: true,               // OpenAI Structured Outputs strict mode
    tags: Some(vec!["image".into()]),  // filter by tags
    prefix: None,               // filter by module ID prefix
})?;
```

**Strict mode** (`strict: true`): sets `additionalProperties: false`, makes all properties required (optional ones become nullable), removes defaults.

**Annotation embedding** (`embed_annotations: true`): appends `[Annotations: read_only, idempotent]` to descriptions.

**Filtering**: `tags` or `prefix` to expose a subset of modules.

## Features

- **Auto-discovery** — all modules in the extensions directory are found and exposed automatically
- **Three transports** — stdio (default, for desktop clients), Streamable HTTP, and SSE
- **JWT authentication** — optional Bearer token auth for HTTP transports with `JWTAuthenticator`, permissive mode, PEM key file support, and env var fallback
- **Approval mechanism** — runtime approval via MCP elicitation, auto-approve, or always-deny handlers
- **AI guidance** — error responses include `retryable`, `ai_guidance`, `user_fixable`, and `suggestion` fields for agent consumption
- **AI intent metadata** — tool descriptions enriched with `x-when-to-use`, `x-when-not-to-use`, `x-common-mistakes`, `x-workflow-hints` from module metadata
- **Extension helpers** — modules can call `report_progress()` and `elicit()` during execution for MCP progress reporting and user input
- **Annotation mapping** — apcore annotations (readonly, destructive, idempotent) map to MCP ToolAnnotations
- **Schema conversion** — JSON Schema `$ref`/`$defs` inlining, strict mode for OpenAI Structured Outputs
- **Error sanitization** — ACL errors and internal errors are sanitized; stack traces are never leaked
- **Dynamic registration** — modules registered/unregistered at runtime are reflected immediately
- **Dual output** — same registry powers both MCP Server and OpenAI tool definitions
- **Tool Explorer** — browser-based UI for browsing schemas and testing tools interactively

## How It Works

### Mapping: apcore to MCP

| apcore | MCP |
|--------|-----|
| `module_id` | Tool name |
| `description` | Tool description |
| `input_schema` | `inputSchema` |
| `annotations.readonly` | `ToolAnnotations.readOnlyHint` |
| `annotations.destructive` | `ToolAnnotations.destructiveHint` |
| `annotations.idempotent` | `ToolAnnotations.idempotentHint` |
| `annotations.open_world` | `ToolAnnotations.openWorldHint` |

### Mapping: apcore to OpenAI Tools

| apcore | OpenAI |
|--------|--------|
| `module_id` (`image.resize`) | `name` (`image-resize`) |
| `description` | `description` |
| `input_schema` | `parameters` |

Module IDs with dots are normalized to dashes for OpenAI compatibility (bijective mapping).

### Architecture

```
Your apcore project (unchanged)
    │
    │  extensions directory
    ▼
apcore-mcp-rust (separate process / library call)
    │
    ├── MCP Server path
    │     SchemaConverter + AnnotationMapper
    │       → MCPServerFactory → ExecutionRouter → TransportManager
    │
    └── OpenAI Tools path
          SchemaConverter + AnnotationMapper + IDNormalizer
            → OpenAIConverter → Vec<Value>
```

## Development

```sh
git clone https://github.com/aiperceivable/apcore-mcp-rust.git
cd apcore-mcp-rust
make setup                       # install toolchain + pre-commit hook
make check                       # run all checks
cargo test                       # 671 tests
```

### Common Commands

| Command | Description |
|---------|-------------|
| `make check` | Run all checks (format, lint, chars, tests) |
| `make test` | Run all tests |
| `make lint` | Run Clippy with warnings-as-errors |
| `make fmt` | Auto-format code |
| `make clean` | Clean build artifacts |

## License

Apache-2.0
