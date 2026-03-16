# Task: integration-tests

## Goal

Write end-to-end integration tests verifying the full `OpenAIConverter` pipeline — from a populated `Registry` through to final OpenAI tool definitions — including strict mode, annotation embedding, and filtering.

## Files Involved

- `src/converters/openai.rs` — Add integration test module `#[cfg(test)]` or `tests/converters/openai_test.rs`

## Steps (TDD-first)

1. **Write integration tests**:
   - `test_e2e_simple_module` — Register a simple module, convert to OpenAI format, verify complete structure:
     ```json
     {
       "type": "function",
       "function": {
         "name": "math-add",
         "description": "Add two numbers",
         "parameters": {"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}}, "required": ["a", "b"]}
       }
     }
     ```
   - `test_e2e_strict_mode_full` — Module with optional properties, `x-llm-description`, and `x-custom` keys. Verify:
     - `x-llm-description` promoted to `description`
     - All `x-*` and `default` keys stripped
     - `additionalProperties: false` set
     - All properties in `required` (sorted)
     - Optional properties have nullable type
     - `function.strict: true` present
   - `test_e2e_annotations_embedded` — Module with destructive annotation. Verify description contains `WARNING: DESTRUCTIVE` text.
   - `test_e2e_multiple_modules_filtered` — Register 3 modules with different tags, filter by tag, verify only matching modules are in output.
   - `test_e2e_prefix_filter` — Register modules with different prefixes, filter by prefix, verify correct subset.
   - `test_e2e_roundtrip_name_normalization` — Verify `ModuleIDNormalizer::denormalize(tool.function.name)` recovers original module ID.
   - `test_e2e_empty_schema` — Module with no input schema produces `{type: "object", properties: {}}` as parameters.

2. **Create test helpers** (if needed):
   - Helper to create a mock `Module` implementation for testing
   - Helper to register modules in a `Registry` for test setup
   - Helper to extract fields from the output `Value` for assertions

3. **Verify output against Python reference**:
   - Where possible, use the same input fixtures as the Python test suite
   - Verify field names, nesting structure, and value types match exactly

4. **Run full test suite** — `cargo test`, `cargo clippy`.

## Acceptance Criteria

- [ ] End-to-end test with simple module produces correct OpenAI format
- [ ] Strict mode integration test covers the full pipeline (llm descriptions, strip, strict)
- [ ] Annotation embedding test verifies description suffix
- [ ] Tag and prefix filtering tests verify correct subset selection
- [ ] Name normalization roundtrip test passes
- [ ] Empty schema edge case handled
- [ ] All tests pass, clippy clean

## Dependencies

- convert-registry

## Estimated Time

1 hour
