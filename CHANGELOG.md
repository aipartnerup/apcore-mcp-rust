# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.11.0] - 2026-03-25

### Added
- **Display overlay in `build_tool()`** — MCP tool name, description, and guidance now sourced from `metadata["display"]["mcp"]` when present.
  - Tool name: `metadata["display"]["mcp"]["alias"]` (pre-sanitized by `DisplayResolver`, already `[a-zA-Z_][a-zA-Z0-9_-]*` and ≤ 64 chars).
  - Tool description: `metadata["display"]["mcp"]["description"]`, with `guidance` appended as `\n\nGuidance: <text>` when set.
  - Falls back to raw `descriptor.module_id` / `descriptor.description` when no display overlay is present.
- `build_tool()` now accepts `name_override` parameter for display overlay tool names.
- `build_tools_with_metadata()` method for resolving display overlay from module metadata.

### Changed
- Dependency recommendation: works best with `apcore-toolkit >= 0.4.0` for `DisplayResolver`.

### Fixed
- README: corrected `mcp.serve(Default::default())` to `mcp.serve()` (zero-argument method).
- README: updated apcore version requirement from `>= 0.13.0` to `>= 0.14.0`.
- docs/features: updated function signatures to use config structs (`ServeConfig`, `AsyncServeConfig`, `OpenAIToolsConfig`).

### Tests
- `TestBuildToolDisplayOverlay` (8 tests): MCP alias used as tool name, MCP description used, guidance appended to description, fallback when no overlay, fallback with empty metadata, name_override direct test, all fields combined.

## [0.10.1] - 2026-03-22

### Changed
- Rebrand: aipartnerup → aiperceivable

## [0.10.0] - 2026-03-17

### Added
- Initial project scaffolding: core modules, CLI, server, authentication, and comprehensive planning.
- **MCP server** with stdio, Streamable HTTP, and SSE transport support.
- **MCPServerFactory** for building tools, resources, and initialization options from an apcore registry.
- **ExecutionRouter** for dispatching tool calls with streaming, progress reporting, elicitation, and output formatting.
- **TransportManager** with health/metrics endpoints and Prometheus observability.
- **RegistryListener** for dynamic tool registration/unregistration via registry events.
- **JWTAuthenticator** with configurable claim mapping, algorithm selection, and key file support.
- **AuthMiddlewareLayer** (Tower layer) for HTTP request authentication with `AUTH_IDENTITY` task-local propagation.
- **Adapters**: AnnotationMapper, SchemaConverter (with `$ref`/`$defs` inlining), ErrorMapper, ModuleIDNormalizer, ElicitationApprovalHandler.
- **OpenAIConverter** for translating apcore registries to OpenAI function-calling format with strict mode support.
- **Explorer UI** powered by `mcp-embedded-ui` crate, with AuthBridge for identity propagation between apcore and the UI layer.
- **CLI** (`apcore-mcp`) with `--transport`, `--host`, `--port`, `--extensions-dir`, `--tags`, `--prefix`, and `--jwt-*` flags.
- **Helper functions**: `report_progress` and `elicit` for MCP progress notifications and user elicitation.
- **Constants**: `ErrorCode` and `RegistryEvent` enums with strum-based serialization matching the Python SDK wire format.
- **APCoreMCPBuilder** for fluent construction with backend, authenticator, metrics, output formatter, and approval handler.
- Convenience functions: `serve()`, `async_serve()`, `to_openai_tools()`.
- `Makefile` with `setup`, `check`, `test`, `lint`, `fmt`, and `clean` targets.
- Git pre-commit hook via `make setup` using `apdev-rs check-chars`.
- 671 tests across unit, integration, and doc-test suites.

### Changed
- `apcore` dependency switched from local path (`../apcore-rust`) to published crate (`apcore = "0.13"`).
- Explorer module refactored: hand-rolled `api.rs` and `templates.rs` replaced by `mcp-embedded-ui = "0.3"` crate with bridge adapters (`AuthBridge`, `wrap_call_fn`).
- `OutputFormatter` type alias uses `Box<dyn Fn>` (Send + Sync) for custom result formatting.
- `StreamResult` type alias introduced for `Pin<Box<dyn Stream<Item = Result<Value, ExecutorError>> + Send>>`.
- `ReadResourceHandler` type alias introduced for the read_resource handler closure.
- `ExecutionRouter::new_with_formatter()` constructor added for creating routers with pre-configured settings but no executor.

### Removed
- `src/explorer/api.rs` — ExplorerState, API handlers, and CallResponse (replaced by `mcp-embedded-ui`).
- `src/explorer/templates.rs` — HTML template rendering (replaced by `mcp-embedded-ui`).

[0.11.0]: https://github.com/aiperceivable/apcore-mcp-rust/compare/v0.10.1...v0.11.0
[0.10.1]: https://github.com/aiperceivable/apcore-mcp-rust/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/aiperceivable/apcore-mcp-rust/releases/tag/v0.10.0
