# Feature: APCoreMCP (Unified API)

## Module Purpose
Provides the high-level unified API for creating and serving an MCP bridge from an apcore extensions directory or backend. Wraps all sub-components (server, auth, adapters, etc.) behind a single builder-pattern struct.

## Public API Surface

### APCoreMCP
- Builder pattern construction with all configuration options
- `serve()` — blocking serve (takes no arguments; configuration is set via the builder)
- `async_serve(config: AsyncServeConfig) -> AsyncIterator` — async context manager equivalent
- `to_openai_tools(embed_annotations, strict) -> Vec<ToolDef>`
- `registry() -> &Registry`
- `executor() -> &Executor`
- `tools() -> Vec<String>`

### Convenience Functions
- `serve(backend: impl Into<BackendSource>, config: ServeConfig)` — one-shot serve
- `async_serve(backend: impl Into<BackendSource>, config: AsyncServeConfig)` — one-shot async serve
- `to_openai_tools(backend: impl Into<BackendSource>, config: OpenAIToolsConfig) -> Vec<ToolDef>`

## Acceptance Criteria
- [ ] APCoreMCP accepts extensions_dir (Path) or backend (Registry/Executor)
- [ ] Builder pattern configures all options (name, version, tags, prefix, etc.)
- [ ] serve() starts blocking server with chosen transport
- [ ] async_serve() returns async context manager-like stream
- [ ] to_openai_tools() delegates to OpenAIConverter
- [ ] registry()/executor()/tools() provide introspection
- [ ] Convenience functions work without constructing APCoreMCP
- [ ] All options have sensible defaults matching Python implementation
