# apcore-mcp-rust

[![Version](https://img.shields.io/crates/v/apcore-mcp)](https://crates.io/crates/apcore-mcp)
[![Coverage](https://img.shields.io/codecov/c/github/aipartnerup/apcore-mcp-rust)](https://codecov.io/gh/aipartnerup/apcore-mcp-rust)
[![License](https://img.shields.io/crates/l/apcore-mcp)](LICENSE)

MCP bridge for apcore -- expose apcore modules as MCP tools.

## Installation

```sh
cargo add apcore-mcp
```

## Quick Start

```rust
use apcore_mcp::APCoreMCP;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let bridge = APCoreMCP::builder()
        .name("my-server")
        .transport("stdio")
        .build()?;

    bridge.serve().await?;
    Ok(())
}
```

## Transport Modes

apcore-mcp supports three transport modes:

- **stdio** -- Standard input/output for CLI-based MCP clients
- **streamable-http** -- HTTP transport with streaming support
- **sse** -- Server-Sent Events for browser and web clients

## Authentication

JWT-based authentication is supported via the `JWTAuthenticator`:

```rust
use apcore_mcp::auth::{JWTAuthenticator, ClaimMapping};

let auth = JWTAuthenticator::new(
    "your-secret-key",
    ClaimMapping::default(),
);

let bridge = APCoreMCP::builder()
    .name("secure-server")
    .transport("streamable-http")
    .authenticator(auth)
    .build()?;
```

## CLI Usage

```sh
# Run with stdio transport (default)
apcore-mcp

# Run with HTTP transport on a specific port
apcore-mcp --transport streamable-http --port 8080

# Run with SSE transport
apcore-mcp --transport sse --port 3000
```

## Documentation

Full documentation is available at <https://apcore.aipartnerup.com/>.
