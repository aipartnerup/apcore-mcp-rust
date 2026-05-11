# Converters Feature — Overview

## Overview

The converters feature provides `OpenAIConverter`, which transforms apcore module registries and descriptors into OpenAI function-calling tool definitions. It composes three existing adapter components (SchemaConverter, AnnotationMapper, ModuleIDNormalizer) and adds OpenAI-specific strict mode logic that enforces `additionalProperties: false`, makes all properties required, and nullifies optional fields.

## Scope

**In scope:**
- `OpenAIConverter` struct with adapter composition
- `convert_registry`: Iterate registry modules with tag/prefix filtering, produce OpenAI tool definitions
- `convert_descriptor`: Convert a single descriptor to `{type: "function", function: {name, description, parameters}}`
- Strict mode algorithm (Algorithm A23): `apply_llm_descriptions`, `strip_extensions`, `convert_to_strict`
- `ConverterError` enum for converter-specific error cases
- Unit tests for all components (TDD-first approach)
- Integration tests with mock Registry

**Out of scope:**
- Adapter implementations (owned by `src/adapters/`)
- Anthropic/Claude format conversion (future feature)
- apcore core types (owned by `apcore-rust` crate)
- Upstream `to_strict_schema()` — reimplemented locally due to missing public API in apcore-rust

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | Edition 2021 | Language |
| `serde` / `serde_json` | 1.x | JSON construction and manipulation |
| `thiserror` | 2.x | Error type derivation for `ConverterError` |
| `apcore` | 0.13 | Registry, ModuleDescriptor, ModuleAnnotations types |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies | Status |
|-------|---------|-------|-----------|--------------|--------|
| 1 | converter-types | Define ConverterError, OpenAIConverter struct, module exports | 1h | none | not started |
| 2 | strict-mode | Implement strict mode algorithm | 2h | converter-types | not started |
| 3 | convert-descriptor | Implement convert_descriptor | 2h | converter-types, strict-mode | not started |
| 4 | convert-registry | Implement convert_registry | 1h | convert-descriptor | not started |
| 5 | integration-tests | End-to-end integration tests | 1h | convert-registry | not started |

Tasks are sequential. Task 2 (strict-mode) can technically be parallelized with task 3 if strict mode is stubbed, but the dependency is logical since convert-descriptor calls strict mode.

## Progress

- [x] Planning complete
- [ ] converter-types
- [ ] strict-mode
- [ ] convert-descriptor
- [ ] convert-registry
- [ ] integration-tests

## Reference Documents

- [Feature Spec](../../docs/features/converters.md)
- [Python Reference Implementation](../../../apcore-mcp-python/src/apcore_mcp/converters/openai.py)
- [Python Strict Mode](../../../apcore-python/src/apcore/schema/strict.py)
- [Rust Stub](../../src/converters/openai.rs)
- [Rust Adapters](../../src/adapters/)
- [apcore Registry](../../../apcore-rust/src/registry/registry.rs)
