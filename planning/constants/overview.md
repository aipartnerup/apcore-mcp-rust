# Constants Feature -- Overview

## Overview

The constants module provides shared, immutable definitions used across the apcore-mcp Rust bridge: standardised error codes, registry event names, and a module ID validation regex. It is the foundation that other modules depend on for consistent naming and validation.

## Scope

### Included

- `ErrorCode` enum with all 18 error codes from the Python reference, including `Display`, `FromStr`, `Serialize`, and `Deserialize` implementations
- `RegistryEvent` enum (`Register`, `Unregister`) with string conversion
- `MODULE_ID_PATTERN` regex constant and a compiled `module_id_regex()` helper
- Unit and integration tests for all public items

### Excluded

- Runtime error handling logic (belongs in the error module)
- Registry implementation (belongs in the server/registry module)
- Module ID parsing beyond regex validation (belongs in the module loader)

## Technology Stack

| Component | Choice |
|-----------|--------|
| Language | Rust (edition 2021) |
| Async runtime | tokio (not directly used here, but project-wide) |
| Serialization | serde + serde_json |
| Schema generation | schemars |
| Enum derive utilities | strum / strum_macros |
| Regex | regex crate |
| Error handling | thiserror (for `FromStr` error type) |
| Test framework | built-in `#[cfg(test)]` + cargo test |

## Task Execution Order

| # | Task File | Description | Status |
|---|-----------|-------------|--------|
| 1 | `tasks/setup.md` | Add strum dependency, clean stub, set up test scaffolding | Not Started |
| 2 | `tasks/error-codes.md` | Implement ErrorCode enum with 18 variants and serde support | Not Started |
| 3 | `tasks/registry-events.md` | Implement RegistryEvent enum with Display/FromStr | Not Started |
| 4 | `tasks/patterns.md` | Define MODULE_ID_PATTERN and module_id_regex() helper | Not Started |
| 5 | `tasks/integration.md` | Cross-cutting integration tests and final verification | Not Started |

## Progress

- **Total tasks:** 5
- **Completed:** 0
- **In progress:** 0
- **Not started:** 5

## Reference Documents

- Feature specification: [`docs/features/constants.md`](../../docs/features/constants.md)
- Python reference implementation: `apcore-mcp-python/src/apcore_mcp/constants.py`
- Cross-language type mapping: `apcore/docs/spec/type-mapping.md`
