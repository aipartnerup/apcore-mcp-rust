# CLI Feature — Overview

## Overview

Port the Python CLI entry point (`__main__.py`) to idiomatic Rust. The module provides command-line argument parsing via clap (derive), tracing initialization, JWT key resolution, approval handler selection, and server startup delegation. This is the user-facing entry point for the `apcore-mcp` binary.

## Scope

- Replace stub code in `src/cli.rs` with full implementation
- Define `Transport`, `ApprovalMode`, and `LogLevel` enums with clap `ValueEnum`
- Implement all CLI arguments matching the Python implementation
- Implement JWT key resolution chain: `--jwt-key-file` > `--jwt-secret` > `JWT_SECRET` env
- Implement tracing subscriber initialization from `--log-level`
- Implement approval handler construction from `--approval` mode
- Validate extensions directory, port range, and name length
- Exit codes: 0=normal, 1=invalid args, 2=startup failure
- Update binary entry point for async main
- Comprehensive unit and integration tests

**Out of scope:** Server implementation (delegated to `serve()`), transport layer, auth middleware (separate features).

## Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Language | Rust | edition 2021 |
| Async runtime | tokio | 1.x |
| Arg parsing | clap (derive) | 4.x |
| Logging | tracing + tracing-subscriber | 0.1.x / 0.3.x |
| JWT auth | jsonwebtoken | 9.x (for key file reading) |
| Approval types | apcore | 0.13.x |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies |
|-------|---------|-------|-----------|--------------|
| 1 | define-cli-enums | Define Transport, ApprovalMode, LogLevel enums | 30 min | none |
| 2 | implement-tracing-init | Implement tracing subscriber initialization | 30 min | none |
| 3 | implement-cli-args | Implement full CliArgs struct with all arguments | 45 min | define-cli-enums |
| 4 | implement-run-function | Implement run() with validation, JWT, approval | 1.5 hr | implement-cli-args, implement-tracing-init |
| 5 | add-cli-unit-tests | Unit tests for parsing, validation, JWT resolution | 1.5 hr | implement-run-function |
| 6 | add-cli-integration-tests | Integration tests for CLI exit codes | 1 hr | add-cli-unit-tests |
| 7 | update-binary-entrypoint | Update bin/apcore-mcp.rs and final cleanup | 20 min | add-cli-integration-tests |

**Note:** Tasks 1 and 2 can run in parallel (no dependencies). Task 3 depends only on task 1.

## Progress

| Task ID | Status |
|---------|--------|
| define-cli-enums | not started |
| implement-tracing-init | not started |
| implement-cli-args | not started |
| implement-run-function | not started |
| add-cli-unit-tests | not started |
| add-cli-integration-tests | not started |
| update-binary-entrypoint | not started |

## Reference Documents

- Feature spec: `docs/features/cli.md`
- Python reference: `apcore-mcp-python/src/apcore_mcp/__main__.py`
- Current Rust stub: `src/cli.rs`
- Binary entry point: `src/bin/apcore-mcp.rs`
- Implementation plan: `planning/cli/plan.md`
