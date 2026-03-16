# Execution Router Feature — Overview

## Overview

The execution router is the central dispatch layer between the MCP server's `call_tool` handler and the apcore executor. It receives tool call requests, constructs per-call execution contexts with MCP callbacks (progress, elicitation), optionally validates inputs, selects between streaming and non-streaming execution paths, formats results for LLM consumption, and maps any errors into MCP-compatible responses. The implementation ports the Python `ExecutionRouter` class to idiomatic Rust using async traits, the `Stream` trait for streaming, and `serde_json::Value` for dynamic JSON handling.

## Scope

**In scope:**
- `Executor` trait defining the async interface to the apcore execution pipeline
- `ExecutionRouter` struct with `handle_call` as the primary entry point
- Per-call `Context` construction with progress and elicit callback injection
- Non-streaming path via `executor.call_async()`
- Streaming path via `executor.stream()` with progress notifications and deep merge
- Pre-execution input validation when `validate_inputs` is enabled
- Output formatting with configurable formatter (default: JSON serialization)
- Error mapping via the existing `ErrorMapper` from `adapters::errors`
- `deep_merge` utility for accumulating stream chunks (depth-capped at 32)
- Unit and integration tests for all paths

**Out of scope:**
- MCP transport/protocol handling (owned by `src/server/transport.rs`)
- Tool discovery and registration (owned by `src/server/factory.rs`)
- Error mapper implementation (owned by `src/adapters/errors.rs`)
- Helper callback types (owned by `src/helpers.rs`)
- Auth/identity extraction from MCP requests (owned by `src/auth/`)

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | Edition 2021 | Language |
| `tokio` | 1.x | Async runtime |
| `tokio-stream` | 0.1.x | `Stream` trait and `StreamExt` for async iteration |
| `serde` / `serde_json` | 1.x | JSON serialization, `Value` manipulation |
| `async-trait` | 0.1 | Async methods in the `Executor` trait |
| `tracing` | 0.1 | Structured logging |
| `thiserror` | 2.x | Executor error types |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies | Status |
|-------|---------|-------|-----------|--------------|--------|
| 1 | executor-trait | Define Executor trait with call_async, stream, validate | 1.5h | none | not started |
| 2 | deep-merge | Implement deep_merge for serde_json::Value | 1h | none | not started |
| 3 | output-formatting | Implement output formatter with JSON fallback | 45min | none | not started |
| 4 | context-construction | Build per-call Context with callback injection | 2h | executor-trait | not started |
| 5 | non-streaming-path | Implement _handle_call_async with error mapping | 1.5h | context-construction, output-formatting | not started |
| 6 | streaming-path | Implement _handle_stream with progress and deep merge | 2h | context-construction, output-formatting, deep-merge | not started |
| 7 | input-validation | Implement pre-execution validation path | 1h | non-streaming-path | not started |
| 8 | handle-call-orchestrator | Wire handle_call to select execution path | 1.5h | non-streaming-path, streaming-path, input-validation | not started |
| 9 | integration-tests | End-to-end tests with mock executor | 2h | handle-call-orchestrator | not started |

Tasks 1-3 can be parallelized. Tasks 5-6 can be parallelized after task 4 completes. Tasks 7-8 are sequential.

## Progress

- [x] Planning complete
- [ ] executor-trait
- [ ] deep-merge
- [ ] output-formatting
- [ ] context-construction
- [ ] non-streaming-path
- [ ] streaming-path
- [ ] input-validation
- [ ] handle-call-orchestrator
- [ ] integration-tests

## Reference Documents

- [Feature Spec](../../docs/features/execution-router.md)
- [Type Mapping Spec](../../../apcore/docs/spec/type-mapping.md)
- [Python Reference Implementation](../../../apcore-mcp-python/src/apcore_mcp/server/router.py)
- [Implementation Plan](./plan.md)
- [Helpers Plan](../helpers/plan.md)
- [Adapters Plan](../adapters/plan.md)
