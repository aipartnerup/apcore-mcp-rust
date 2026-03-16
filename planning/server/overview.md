# Server Feature — Overview

## Overview

Port the Python MCPServer and RegistryListener to idiomatic Rust. MCPServer is a non-blocking orchestrator that combines factory, router, transport, and listener into a managed server lifecycle with background task execution via `tokio::spawn`. RegistryListener watches an apcore Registry for tool registration events and maintains a thread-safe tool map.

## Scope

- Replace stub code in `src/server/server.rs` with a full MCPServer implementation
- Replace stub code in `src/server/listener.rs` with a full RegistryListener implementation
- Introduce `TransportKind` enum to replace stringly-typed transport selection
- Introduce `MCPServerConfig` struct to collect optional constructor parameters
- Implement start/wait/stop lifecycle using `tokio::spawn`, `oneshot`, and shutdown signaling
- Implement thread-safe tool map in RegistryListener using `RwLock<HashMap>`
- Comprehensive unit and integration tests

**Out of scope:** Implementation of MCPServerFactory, ExecutionRouter, TransportManager (separate features). Auth middleware construction (depends on auth feature).

## Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Language | Rust | edition 2021 |
| Async runtime | tokio | 1.x |
| Serialization | serde + serde_json | 1.x |
| HTTP framework | axum | 0.8.x |
| Logging | tracing | 0.1.x |
| Core types | apcore | 0.13.x |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies |
|-------|---------|-------|-----------|--------------|
| 1 | define-transport-kind-enum | Define TransportKind enum and address formatting | 30 min | none |
| 2 | implement-registry-listener | Implement RegistryListener with RwLock tool map | 1.5 hr | none |
| 3 | define-server-config | Define MCPServerConfig with builder defaults | 45 min | define-transport-kind-enum |
| 4 | implement-mcp-server-struct | Implement MCPServer struct and constructor | 1 hr | define-transport-kind-enum, define-server-config, implement-registry-listener |
| 5 | implement-server-lifecycle | Implement start/wait/stop with tokio::spawn | 2 hr | implement-mcp-server-struct |
| 6 | add-registry-listener-tests | Unit tests for RegistryListener | 1 hr | implement-registry-listener |
| 7 | add-server-unit-tests | Unit tests for MCPServer lifecycle | 1.5 hr | implement-server-lifecycle |
| 8 | add-integration-tests | End-to-end server start/stop tests | 1 hr | add-registry-listener-tests, add-server-unit-tests |
| 9 | update-module-exports | Clean up mod.rs and remove stubs | 20 min | add-integration-tests |

**Note:** Tasks 1 and 2 can run in parallel (no dependencies on each other). Tasks 6 can start as soon as task 2 completes. Tasks 6 and 7 can run in parallel.

## Progress

| Task ID | Status |
|---------|--------|
| define-transport-kind-enum | not started |
| define-server-config | not started |
| implement-registry-listener | not started |
| implement-mcp-server-struct | not started |
| implement-server-lifecycle | not started |
| add-registry-listener-tests | not started |
| add-server-unit-tests | not started |
| add-integration-tests | not started |
| update-module-exports | not started |

## Reference Documents

- Feature spec: `docs/features/server.md`
- Type mapping spec: `apcore/docs/spec/type-mapping.md`
- Python reference: `apcore-mcp-python/src/apcore_mcp/server/server.py`
- Python reference: `apcore-mcp-python/src/apcore_mcp/server/listener.py`
- Implementation plan: `planning/server/plan.md`
