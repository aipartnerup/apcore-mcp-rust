# Task: strict-mode

## Goal

Implement the OpenAI strict mode algorithm (Algorithm A23) as private methods on `OpenAIConverter`. This includes `_apply_llm_descriptions`, `_strip_extensions`, and `_convert_to_strict` — matching the behavior of Python's `apcore.schema.strict` module.

## Files Involved

- `src/converters/openai.rs` — Add private strict mode methods

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_apply_llm_descriptions_replaces_description` — When both `description` and `x-llm-description` exist, description is replaced
   - `test_apply_llm_descriptions_preserves_when_no_llm` — When only `description` exists, it is preserved
   - `test_apply_llm_descriptions_nested_properties` — Recursion into `properties`
   - `test_apply_llm_descriptions_nested_items` — Recursion into array `items`
   - `test_apply_llm_descriptions_nested_oneof` — Recursion into `oneOf`/`anyOf`/`allOf`
   - `test_apply_llm_descriptions_nested_defs` — Recursion into `$defs` and `definitions`
   - `test_strip_extensions_removes_x_keys` — All `x-*` keys are removed
   - `test_strip_extensions_removes_defaults` — `default` keys are removed
   - `test_strip_extensions_recursive` — Strips in nested objects and arrays
   - `test_strip_extensions_preserves_non_x_keys` — Non-extension keys are preserved
   - `test_convert_to_strict_sets_additional_properties_false` — Object with properties gets `additionalProperties: false`
   - `test_convert_to_strict_makes_all_required` — All properties added to `required`, sorted alphabetically
   - `test_convert_to_strict_nullable_optional_string` — Optional string property becomes `["string", "null"]`
   - `test_convert_to_strict_nullable_optional_array_type` — Optional property with array type gets `"null"` appended
   - `test_convert_to_strict_nullable_optional_ref` — Optional property without `type` gets wrapped in `oneOf` with null
   - `test_convert_to_strict_preserves_required` — Already-required properties are not made nullable
   - `test_convert_to_strict_recursive_nested_object` — Recurses into nested object properties
   - `test_convert_to_strict_recursive_items` — Recurses into array `items`
   - `test_convert_to_strict_recursive_oneof` — Recurses into `oneOf`/`anyOf`/`allOf`
   - `test_apply_strict_mode_full_pipeline` — End-to-end: llm descriptions + strip + strict on a realistic schema

2. **Implement `apply_llm_descriptions`**:
   ```rust
   fn apply_llm_descriptions(node: &mut Value) {
       // If both "x-llm-description" and "description" exist, replace description
       // Recurse into properties, items, oneOf/anyOf/allOf, $defs/definitions
   }
   ```

3. **Implement `strip_extensions`**:
   ```rust
   fn strip_extensions(node: &mut Value) {
       // Remove all keys starting with "x-" and "default" keys
       // Recurse into all nested dicts and arrays
   }
   ```

4. **Implement `convert_to_strict`**:
   ```rust
   fn convert_to_strict(node: &mut Value) {
       // For objects with properties:
       //   Set additionalProperties: false
       //   Identify optional properties (not in existing required)
       //   Make optional properties nullable
       //   Set required to sorted list of all property names
       // Recurse into properties, items, oneOf/anyOf/allOf, $defs/definitions
   }
   ```

5. **Wire into `_apply_strict_mode`**:
   ```rust
   fn _apply_strict_mode(&self, schema: &Value) -> Value {
       let mut schema = schema.clone();
       Self::apply_llm_descriptions(&mut schema);
       Self::strip_extensions(&mut schema);
       Self::convert_to_strict(&mut schema);
       schema
   }
   ```

6. **Run tests** — ensure all pass. Run `cargo clippy`.

## Implementation Notes

- All three helper functions mutate `&mut Value` in place (matching Python's in-place mutation pattern)
- `_apply_strict_mode` clones first, then mutates (matching Python's `copy.deepcopy` + mutate)
- Use `Value::as_object_mut()` to access map entries and `retain()` or manual key removal for stripping
- For nullable wrapping: when a property has no `type` key (e.g., pure `$ref`), wrap in `{"oneOf": [original, {"type": "null"}]}`

## Acceptance Criteria

- [ ] `x-llm-description` replaces `description` where both exist
- [ ] All `x-*` keys and `default` keys are stripped recursively
- [ ] Object schemas get `additionalProperties: false`
- [ ] All properties become required (sorted alphabetically)
- [ ] Optional properties become nullable (type array or oneOf wrapping)
- [ ] Already-required properties are not double-nullified
- [ ] Recursion covers properties, items, oneOf/anyOf/allOf, $defs/definitions
- [ ] All tests pass, clippy clean

## Dependencies

- converter-types

## Estimated Time

2 hours
