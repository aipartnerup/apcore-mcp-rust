# APCoreMCP Feature Overview

## Summary
Port the Python `APCoreMCP` unified API class to Rust. This is the high-level entry point that wires an apcore `Registry` + `Executor` to an MCP server via builder-pattern construction, with `serve()`, `async_serve()`, and `to_openai_tools()` methods plus convenience free functions.

## Python Source Analysis
- **File**: `apcore_mcp.py` (~400 LOC, MEDIUM complexity)
- **Class**: `APCoreMCP` — polymorphic constructor accepting `str | Path | Registry | Executor`
- **Key patterns**:
  - Constructor validates inputs, resolves backend via `resolve_registry()` / `resolve_executor()`
  - `_build_server_components()` is shared by `serve()` and `async_serve()`
  - `serve()` is blocking (wraps `asyncio.run`); `async_serve()` is an async context manager yielding a Starlette app
  - `to_openai_tools()` delegates to `OpenAIConverter`
  - Lazy imports used to break circular dependencies (not needed in Rust)
  - Three convenience free functions mirror the class methods

## Rust Target Architecture

### Input Type Enum
Replace Python's duck-typed polymorphic constructor with a Rust enum:
```rust
pub enum BackendSource {
    ExtensionsDir(PathBuf),
    Registry(Arc<Registry>),
    Executor(Arc<Executor>),
}
```
The builder accepts `BackendSource` via `impl Into<BackendSource>` conversions.

### Struct Layout
```rust
pub struct APCoreMCP {
    registry: Arc<Registry>,
    executor: Arc<Executor>,
    config: APCoreMCPConfig,
}
```
`APCoreMCPConfig` holds all optional parameters (name, version, tags, prefix, validate_inputs, auth settings, etc.).

### Builder Pattern
`APCoreMCPBuilder` uses the existing pattern but adds:
- `backend(impl Into<BackendSource>)` — required
- Missing optional setters: `version`, `tags`, `prefix`, `log_level`, `metrics_collector`, `output_formatter`, `require_auth`, `exempt_paths`, `approval_handler`
- `build()` resolves the backend source into `Arc<Registry>` + `Arc<Executor>` and validates config

### Serve Methods
- `serve()` — blocks via `tokio::runtime::Runtime::block_on(self.async_serve_internal())`
- `async_serve()` — returns a handle/guard that yields the axum `Router` for embedding
- Both call a shared `build_server_components()` method

### Convenience Functions
Three free functions (`serve`, `async_serve`, `to_openai_tools`) construct a temporary `APCoreMCP` internally.

## Key Dependencies
- `apcore` crate: `Registry`, `Executor`, `ApprovalHandler`
- `tokio`: async runtime
- `axum`: HTTP server (replaces Starlette)
- `serde` / `serde_json`: serialization
- `tracing`: logging (replaces Python `logging`)

## Files Modified
- `src/apcore_mcp.rs` — primary implementation (rewrite)
- `src/utils.rs` — update `resolve_registry` / `resolve_executor` signatures
- `src/lib.rs` — update re-exports if needed

## Differences from Python
| Aspect | Python | Rust |
|--------|--------|------|
| Polymorphic input | duck typing (`isinstance`) | `BackendSource` enum + `From` impls |
| Lazy imports | needed for circular deps | not needed (module system prevents) |
| Async context manager | `@asynccontextmanager` yields Starlette | returns guard/handle with axum Router |
| Error handling | exceptions | `Result<T, APCoreMCPError>` with `thiserror` |
| Blocking serve | `asyncio.run()` | `tokio::runtime::Runtime::block_on()` |
| Callbacks | `Callable` | `Box<dyn Fn() + Send + Sync>` |
| Output formatter | `Callable[[dict], str]` | `Box<dyn Fn(&Value) -> String + Send + Sync>` |
