# Explorer Feature — Overview

## Overview

Port the Python MCP Tool Explorer to Rust. The explorer is a browser-based UI that lists registered MCP tools with their descriptions and input schemas, and optionally allows interactive tool execution. In Python this is a thin wrapper over the `mcp-embedded-ui` library; in Rust we build equivalent functionality as an axum `Router` with embedded HTML and a JSON API.

## Scope

- Replace stub code in `src/explorer/mount.rs` with a full implementation
- Define `ExplorerConfig`, `ToolInfo`, and `HandleCallFn` types
- Embed a minimal HTML/JS explorer page served via `include_str!`
- Implement JSON API endpoints: `GET /tools` and `POST /tools/:name/call`
- Bridge `AUTH_IDENTITY` task-local for authenticated tool execution
- Comprehensive unit and integration tests

**Out of scope:** Rich UI framework (React, etc.), WebSocket-based streaming, tool approval flows in the UI.

## Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Language | Rust | edition 2021 |
| Async runtime | tokio | 1.x |
| HTTP framework | axum | 0.8.x |
| Middleware | tower | 0.5.x |
| Serialization | serde + serde_json | 1.x |
| Auth | AUTH_IDENTITY task-local | (internal) |
| Logging | tracing | 0.1.x |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies |
|-------|---------|-------|-----------|--------------|
| 1 | define-explorer-types | Define ExplorerConfig, ToolInfo, HandleCallFn types | 45 min | none |
| 2 | implement-html-template | Create embedded HTML/JS explorer page | 1 hr | none |
| 3 | implement-api-handlers | Implement GET /tools and POST /tools/:name/call | 1.5 hr | define-explorer-types |
| 4 | implement-explorer-mount | Wire create_explorer_mount with config, auth, Router | 1.5 hr | implement-api-handlers, implement-html-template |
| 5 | add-explorer-tests | Unit and integration tests for all endpoints | 1.5 hr | implement-explorer-mount |
| 6 | update-module-exports | Clean up mod.rs and remove stubs | 20 min | add-explorer-tests |

**Note:** Tasks 1 and 2 can run in parallel (no dependencies between them). Task 3 depends only on task 1.

## Progress

| Task ID | Status |
|---------|--------|
| define-explorer-types | not started |
| implement-html-template | not started |
| implement-api-handlers | not started |
| implement-explorer-mount | not started |
| add-explorer-tests | not started |
| update-module-exports | not started |

## Reference Documents

- Feature spec: `docs/features/explorer.md`
- Python reference: `apcore-mcp-python/src/apcore_mcp/explorer/__init__.py`
- Implementation plan: `planning/explorer/plan.md`
- Auth middleware: `src/auth/middleware.rs`
- Execution router: `src/server/router.rs`
