# Helpers Feature — Overview

## Overview

The helpers module provides MCP-specific utility functions for progress reporting and user elicitation within apcore module execution contexts. It ports the Python `helpers.py` module to idiomatic Rust, using async callback trait objects stored in the execution context's data map, with graceful no-op behavior when callbacks are absent (supporting non-MCP execution paths).

## Scope

- Define `ElicitAction` enum and `ElicitResult` struct with serde serialization
- Define async callback type aliases (`ProgressCallback`, `ElicitCallback`)
- Define well-known context data key constants (`MCP_PROGRESS_KEY`, `MCP_ELICIT_KEY`)
- Implement `report_progress` with callback lookup and no-op fallback
- Implement `elicit` with callback lookup and `None` fallback
- Unit tests for all types and functions
- Integration smoke test for end-to-end validation

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | Edition 2021 | Language |
| tokio | 1.x | Async runtime |
| serde / serde_json | 1.x | Serialization of `ElicitAction`, `ElicitResult`, and `Value` |
| schemars | 0.8 | JSON Schema generation for types |
| tracing | 0.1 | Debug-level logging for downcast failures |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies |
|-------|---------|-------|-----------|--------------|
| 1 | define-callback-types | Define callback type aliases and finalize ElicitAction/ElicitResult types | 30 min | — |
| 2 | define-constants | Define MCP_PROGRESS_KEY and MCP_ELICIT_KEY constants | 10 min | — |
| 3 | implement-report-progress | Implement report_progress with context callback lookup | 45 min | define-callback-types, define-constants |
| 4 | implement-elicit | Implement elicit with context callback lookup | 45 min | define-callback-types, define-constants |
| 5 | unit-tests | Comprehensive unit tests for all types and functions | 60 min | implement-report-progress, implement-elicit |
| 6 | integration-smoke-test | End-to-end smoke test with mock context | 30 min | unit-tests |

Tasks 1-2 can run in parallel. Tasks 3-4 can run in parallel (after 1-2). Tasks 5-6 are sequential.

**Total estimated time: ~3.5 hours**

## Progress

| Task ID | Status |
|---------|--------|
| define-callback-types | not started |
| define-constants | not started |
| implement-report-progress | not started |
| implement-elicit | not started |
| unit-tests | not started |
| integration-smoke-test | not started |

## Reference Documents

- Feature spec: `docs/features/helpers.md`
- Type mapping spec: `apcore/docs/spec/type-mapping.md`
- Python reference implementation: `apcore-mcp-python/src/apcore_mcp/helpers.py`
- Existing Rust stub: `src/helpers.rs`
- Implementation plan: `planning/helpers/plan.md`
