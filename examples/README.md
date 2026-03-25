# Examples

Runnable demos of **apcore-mcp** (Rust) with the Tool Explorer UI.

```
examples/
├── run.rs                     # Unified launcher
├── extensions/                # apcore modules (auto-discovered)
│   ├── text_echo.rs           # (planned) Echo text back
│   ├── math_calc.rs           # (planned) Basic arithmetic
│   └── greeting.rs            # (planned) Personalized greetings
└── README.md
```

## Quick Start

```bash
# From the project root
cargo run --example run
```

Open http://127.0.0.1:8000/explorer/ to see the Tool Explorer UI.

## What the Examples Demonstrate

| Module | Description |
|--------|-------------|
| `text_echo` | Echo input text back, optionally converting to uppercase |
| `math_calc` | Basic arithmetic: add, subtract, multiply, divide |
| `greeting` | Personalized greeting in different styles (friendly, formal, pirate) |

These modules mirror the Python examples in `apcore-mcp-python/examples/extensions/` and illustrate how to:

1. Define apcore modules using the Rust `Module` trait
2. Auto-discover modules from an `extensions/` directory
3. Expose them as MCP tools via `APCoreMCP`
4. Enable the browser-based Explorer UI for interactive testing

## Testing with an MCP Client

### Explorer UI

The built-in Explorer UI at http://127.0.0.1:8000/explorer/ lets you browse tools, view schemas, and execute tools interactively from the browser.

### cURL

```bash
# Health check
curl http://localhost:8000/health

# List available tools (MCP initialize)
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}'
```

## Architecture

The example uses the `APCoreMCP` builder pattern:

```rust
let mcp = APCoreMCP::builder()
    .backend(registry)               // apcore Registry with discovered modules
    .name("apcore-mcp-examples")
    .transport("streamable-http")
    .host("127.0.0.1")
    .port(8000)
    .include_explorer(true)
    .build()?;

mcp.serve_with_options(ServeOptions {
    explorer: true,
    explorer_prefix: "/explorer".to_string(),
    ..Default::default()
})?;
```

This mirrors the Python `serve()` call:

```python
serve(registry, transport="streamable-http", port=8000, explorer=True, allow_execute=True)
```
