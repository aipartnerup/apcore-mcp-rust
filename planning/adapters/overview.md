# Adapters Feature — Overview

## Overview

The adapters feature provides a translation layer between apcore domain types and MCP (Model Context Protocol) wire types. It consists of five adapter modules that handle annotation mapping, error transformation, JSON Schema conversion, module ID normalization, and user approval via MCP elicitation. Each adapter is stateless and designed for zero-allocation overhead where possible.

## Scope

**In scope:**
- `AnnotationMapper`: Convert `ModuleAnnotations` to MCP `ToolAnnotations` and generate description suffixes
- `ErrorMapper`: Convert `ModuleError` to MCP error response dicts with sanitization and AI guidance
- `SchemaConverter`: Inline `$ref` references, strip `$defs`, enforce root `type: "object"` in JSON Schemas
- `ModuleIDNormalizer`: Bijective dot-to-dash mapping with regex validation for module IDs
- `ElicitationApprovalHandler`: Implement `ApprovalHandler` trait by bridging to MCP elicit callback
- `AdapterError` enum for adapter-specific error cases
- Unit tests for all adapters (TDD-first approach)

**Out of scope:**
- MCP transport layer (handled by `src/server/`)
- OpenAI format conversion (handled by `src/converters/`)
- apcore core types (owned by `apcore-rust` crate)

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | Edition 2021 | Language |
| `serde` / `serde_json` | 1.x | JSON serialization, camelCase rename, `Value` traversal |
| `schemars` | 0.8 | JSON Schema generation (for typed MCP structs) |
| `tokio` | 1.x | Async runtime for approval handler |
| `async-trait` | 0.1 | Async trait support for `ApprovalHandler` |
| `thiserror` | 2.x | Ergonomic error type derivation |
| `regex` | 1.x | Module ID pattern validation |
| `tracing` | 0.1 | Structured logging in approval handler |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies | Status |
|-------|---------|-------|-----------|--------------|--------|
| 1 | adapter-setup | Scaffold shared types, AdapterError, module re-exports | 1h | none | not started |
| 2 | annotations-mapper | Implement AnnotationMapper | 2h | adapter-setup | not started |
| 3 | error-mapper | Implement ErrorMapper | 3h | adapter-setup | not started |
| 4 | schema-converter | Implement SchemaConverter | 3h | adapter-setup | not started |
| 5 | id-normalizer | Implement ModuleIDNormalizer | 1h | adapter-setup | not started |
| 6 | approval-handler | Implement ElicitationApprovalHandler | 2h | adapter-setup, annotations-mapper | not started |

Tasks 2-5 can be parallelized after task 1 is complete. Task 6 depends on task 2 (uses `AnnotationMapper::has_requires_approval` conceptually, and shares the module structure established in setup).

## Progress

- [x] Planning complete
- [ ] adapter-setup
- [ ] annotations-mapper
- [ ] error-mapper
- [ ] schema-converter
- [ ] id-normalizer
- [ ] approval-handler

## Reference Documents

- [Feature Spec](../../docs/features/adapters.md)
- [Type Mapping Spec](../../../apcore/docs/spec/type-mapping.md)
- [Python Reference Implementation](../../../apcore-mcp-python/src/apcore_mcp/adapters/)
- [apcore Rust Error Types](../../../apcore-rust/src/errors.rs)
- [apcore Rust Module Types](../../../apcore-rust/src/module.rs)
- [apcore Rust Approval Types](../../../apcore-rust/src/approval.rs)
