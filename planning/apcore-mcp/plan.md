# APCoreMCP Implementation Plan

## Execution Order
1. `backend-source-enum` — Define the `BackendSource` enum and `From` impls
2. `config-and-error-types` — Expand `APCoreMCPConfig` and define `APCoreMCPError`
3. `resolve-utils` — Update `resolve_registry` / `resolve_executor` to work with real types
4. `builder-pattern` — Rewrite `APCoreMCPBuilder` with all setters and validation
5. `struct-and-accessors` — Implement `APCoreMCP` struct, `registry()`, `executor()`, `tools()`
6. `build-server-components` — Implement the shared `build_server_components()` method
7. `serve-methods` — Implement `serve()` and `async_serve()`
8. `to-openai-tools` — Implement `to_openai_tools()` delegation
9. `convenience-functions` — Implement the three free functions
10. `integration-wiring` — Update `lib.rs` re-exports and ensure compilation

## Dependency Graph
```
backend-source-enum
        |
config-and-error-types
        |
   resolve-utils
        |
  builder-pattern
        |
struct-and-accessors
        |
build-server-components
       / \
serve-methods  to-openai-tools
       \ /
convenience-functions
        |
integration-wiring
```

## Test Strategy
All tasks follow TDD: write tests first, then implement to make them pass.
- Unit tests in `#[cfg(test)] mod tests` within `src/apcore_mcp.rs`
- Integration tests may go in `tests/` directory
- Mock `Registry` and `Executor` where needed using trait objects or test doubles

## Risk Areas
- The `apcore` crate's `Registry` and `Executor` concrete types may require wrapping in trait objects for testability
- `async_serve` returning an embeddable app differs structurally from Python's async context manager
- Explorer and auth middleware integration depend on those features being ported first (stub for now)
