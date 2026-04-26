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
APCORE_JWT_SECRET=my-secret cargo run --example run
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

---

## Scenario coverage notes (cross-language)

Python and TypeScript ship two additional example scenarios that the Rust
crate does not yet provide:

| Scenario | Python | TypeScript | Rust |
|---|---|---|---|
| `run` (default unified demo) | ✓ | ✓ | ✓ |
| `extensions/` (filesystem-discovered modules) | ✓ | ✓ | ⚠ pending |
| `binding_demo/` (`.binding.yaml`-driven, zero-code-intrusion modules) | ✓ | ✓ | ⚠ pending |

The patterns are exercised by the `run` example above:

- `examples/run/modules.rs` registers modules in-process — equivalent to
  what `extensions/` would discover from a directory. The Rust apcore
  Registry supports filesystem discovery via `Registry::discover_dir`;
  a dedicated `extensions/` example would just narrow the launcher to
  call `discover_dir` instead of inline registration.
- `binding_demo/` requires `apcore-toolkit`'s `BindingLoader::load_binding_dir`
  applied to a directory of `.binding.yaml` files. A Rust port is
  tracked as a follow-up — the Rust `BindingLoader` API surface
  (`apcore-toolkit = 0.5.0`) is available but not yet wired into a
  full demo.

Cross-reference: the equivalent Python examples live at
`apcore-mcp-python/examples/extensions/` and
`apcore-mcp-python/examples/binding_demo/`; TypeScript at
`apcore-mcp-typescript/examples/extensions/` and
`apcore-mcp-typescript/examples/binding_demo/`. A Rust port aiming for
1:1 scenario parity should mirror those structures. [B-006, B-007]
