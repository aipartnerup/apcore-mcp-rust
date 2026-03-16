# Transport Feature — Overview

## Overview

Port the Python `TransportManager` to idiomatic Rust. The module manages three MCP transport modes (stdio, streamable-http, SSE) and provides health/metrics HTTP endpoints on all HTTP transports. Uses tokio for async concurrency, axum for HTTP routing, and tower for middleware composition.

## Scope

- Replace all stub code in `src/server/transport.rs` with full implementations
- Define a `TransportError` error enum for all transport failure modes
- Refactor `TransportManager` to accept an external `MetricsExporter` (remove self-impl)
- Implement health and metrics endpoint handlers
- Implement stdio transport with tokio async I/O
- Implement streamable-http transport with axum Router and `/mcp` mount
- Implement SSE transport (deprecated) with `/sse` and `/messages/` endpoints
- Provide `build_streamable_http_app()` for embedding into larger applications
- Comprehensive unit and integration tests

**Out of scope:** MCP JSON-RPC protocol implementation (assumed as a trait/interface boundary), TLS termination, authentication middleware (handled by auth module).

## Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Language | Rust | edition 2021 |
| Async runtime | tokio | 1.x |
| HTTP framework | axum | 0.8.x |
| Middleware | tower | 0.5.x |
| HTTP utilities | tower-http | 0.6.x |
| Low-level HTTP | hyper | 1.x |
| Serialization | serde + serde_json | 1.x |
| Error handling | thiserror | 2.x |
| UUIDs | uuid | 1.x |
| Logging | tracing | 0.1.x |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies |
|-------|---------|-------|-----------|--------------|
| 1 | define-transport-error | Define TransportError enum with thiserror | 30 min | none |
| 2 | refactor-transport-manager-struct | Refactor TransportManager struct fields and constructor | 30 min | none |
| 3 | implement-health-metrics | Implement health and metrics endpoint handlers | 45 min | define-transport-error, refactor-transport-manager-struct |
| 4 | implement-stdio-transport | Implement run_stdio with tokio stdin/stdout | 1.5 hr | implement-health-metrics |
| 5 | implement-streamable-http-transport | Implement run_streamable_http and build_streamable_http_app | 2 hr | implement-health-metrics |
| 6 | implement-sse-transport | Implement run_sse with SSE endpoints (deprecated) | 1.5 hr | implement-health-metrics |
| 7 | add-transport-unit-tests | Unit tests for all transport components | 1.5 hr | implement-stdio-transport, implement-streamable-http-transport, implement-sse-transport |
| 8 | add-transport-integration-tests | Integration tests: HTTP bind, health, metrics | 1.5 hr | add-transport-unit-tests |
| 9 | cleanup-and-exports | Remove stubs, allow(unused), update mod.rs | 20 min | add-transport-integration-tests |

**Note:** Tasks 4, 5, and 6 can run in parallel after task 3 completes. Tasks 1 and 2 can also run in parallel.

## Progress

| Task ID | Status |
|---------|--------|
| define-transport-error | not started |
| refactor-transport-manager-struct | not started |
| implement-health-metrics | not started |
| implement-stdio-transport | not started |
| implement-streamable-http-transport | not started |
| implement-sse-transport | not started |
| add-transport-unit-tests | not started |
| add-transport-integration-tests | not started |
| cleanup-and-exports | not started |

## Reference Documents

- Feature spec: `docs/features/transport.md`
- Type mapping spec: `apcore/docs/spec/type-mapping.md`
- Python reference implementation: `apcore-mcp-python/src/apcore_mcp/server/transport.py`
- Existing Rust stub: `src/server/transport.rs`
- Implementation plan: `planning/transport/plan.md`
