# Examples

Runnable demos of **apcore-mcp** (Rust) with the Tool Explorer UI.

```
examples/
├── run/
│   ├── main.rs          # Unified launcher (registers modules, starts server)
│   └── modules.rs       # 3 example apcore modules
└── README.md
```

## Quick Start

```bash
# From the project root
cargo run --example run
```

Open http://127.0.0.1:8000/explorer to see the Tool Explorer UI.

### Enable JWT Authentication

```bash
JWT_SECRET=my-secret cargo run --example run
```

The server will print a sample token you can use for testing:

```bash
# Health check (always exempt)
curl http://localhost:8000/health

# Without token → 401
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'

# With token → 200
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <sample-token>" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

## Example Modules

| Module | Description | Tags |
|--------|-------------|------|
| `text.echo` | Echo input text back, optionally converting to uppercase | text, utility |
| `math.calc` | Basic arithmetic: add, subtract, multiply, divide | math, utility |
| `greeting` | Personalized greeting in different styles (friendly, formal, pirate) | text, fun |

These modules mirror the Python examples in `apcore-mcp-python/examples/extensions/` and illustrate how to:

1. Implement the `apcore::Module` trait in Rust
2. Register modules with a `Registry` and create an `Executor`
3. Expose them as MCP tools via `APCoreMCP` builder
4. Enable the Explorer UI for interactive testing
5. Optionally enable JWT authentication

## Testing with cURL

```bash
# Health check
curl http://localhost:8000/health

# List tools
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'

# Call text.echo
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"text.echo","arguments":{"text":"hello","uppercase":true}}}'

# Call math.calc
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"math.calc","arguments":{"a":10,"b":3,"op":"mul"}}}'

# Call greeting
curl -X POST http://localhost:8000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"greeting","arguments":{"name":"World","style":"pirate"}}}'
```

## Architecture

```rust
// Create registry and register modules
let mut registry = Registry::new();
registry.register("text.echo", Box::new(TextEcho), descriptor)?;

// Create executor (owns registry)
let executor = Arc::new(Executor::new(registry, Config::default()));

// Build and serve
let mcp = APCoreMCP::builder()
    .backend(executor)
    .transport("streamable-http")
    .port(8000)
    .include_explorer(true)
    .allow_execute(true)
    .build()?;

mcp.serve()?;
```

This mirrors the Python example:

```python
registry = Registry(extensions_dir="./examples/extensions")
registry.discover()
serve(registry, transport="streamable-http", port=8000, explorer=True, allow_execute=True)
```
