# Task: schema-converter

## Goal

Implement `SchemaConverter` that converts apcore JSON Schemas to MCP-compatible schemas by inlining `$ref` references (up to depth 32), stripping `$defs`, and ensuring the root has `type: "object"`.

## Files Involved

- `src/adapters/schema.rs` — Full implementation replacing the current stub
- `tests/adapters/schema_test.rs` or inline `#[cfg(test)]` module

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_empty_schema` — `{}` returns `{"type": "object", "properties": {}}`
   - `test_null_schema` — `Value::Null` returns default empty object schema
   - `test_simple_object_passthrough` — Schema with `type: "object"` and `properties` passes through unchanged
   - `test_missing_type_gets_object` — Schema with `properties` but no `type` gets `type: "object"` added
   - `test_inline_simple_ref` — Schema with `$ref: "#/$defs/Foo"` and `$defs.Foo` gets Foo inlined
   - `test_inline_nested_refs` — `$ref` that itself contains another `$ref` both get resolved
   - `test_inline_ref_in_properties` — `$ref` inside a property value gets inlined
   - `test_inline_ref_in_array_items` — `$ref` inside `items` gets inlined
   - `test_defs_stripped_after_inlining` — Final output has no `$defs` key
   - `test_circular_ref_detected` — Self-referencing `$ref` returns error
   - `test_depth_exceeded` — Deeply nested refs (> 32 levels) return error
   - `test_diamond_ref_allowed` — Two properties referencing the same `$def` both get inlined (not treated as circular)
   - `test_ref_not_found` — `$ref` pointing to missing def returns error
   - `test_unsupported_ref_format` — `$ref` not starting with `#/$defs/` returns error
   - `test_preserves_additional_properties` — `additionalProperties`, `required`, etc. are preserved
   - `test_convert_input_schema` — End-to-end with a descriptor-like `Value`
   - `test_convert_output_schema` — End-to-end with output schema

2. **Define constant**:
   ```rust
   const MAX_REF_DEPTH: usize = 32;
   ```

3. **Implement `SchemaConverter`**:
   - Change API to accept `&Value` (the schema directly) and return `Result<Value, AdapterError>`
   - `convert_input_schema(&self, schema: &Value) -> Result<Value, AdapterError>`
   - `convert_output_schema(&self, schema: &Value) -> Result<Value, AdapterError>`
   - Both delegate to `convert_schema`

4. **Implement `convert_schema`**:
   ```rust
   fn convert_schema(&self, schema: &Value) -> Result<Value, AdapterError> {
       // Clone to avoid mutating input
       let mut schema = schema.clone();
       // Handle empty/null
       if schema.is_null() || schema.as_object().map_or(false, |m| m.is_empty()) {
           return Ok(json!({"type": "object", "properties": {}}));
       }
       // Inline $refs if $defs present
       if let Some(defs) = schema.get("$defs").cloned() {
           schema = self.inline_refs(&schema, &defs, &HashSet::new(), 0)?;
           schema.as_object_mut().unwrap().remove("$defs");
       }
       // Ensure root type: object
       self.ensure_object_type(&mut schema);
       Ok(schema)
   }
   ```

5. **Implement `inline_refs`** (recursive):
   - Signature: `fn inline_refs(&self, schema: &Value, defs: &Value, seen: &HashSet<String>, depth: usize) -> Result<Value, AdapterError>`
   - If depth > MAX_REF_DEPTH, return `Err(AdapterError::SchemaConversion(...))`
   - If `Value::Object`:
     - If contains `$ref` key: resolve ref, check circular (seen set), recurse on resolved
     - Otherwise: recurse on each value, skip `$defs` key
   - If `Value::Array`: recurse on each element
   - Otherwise: return clone

6. **Implement `resolve_ref`**:
   - Parse `$ref` path: must start with `#/$defs/`
   - Extract def name, look up in `defs` object
   - Return clone of the definition
   - Return `Err` if not found or unsupported format

7. **Implement `ensure_object_type`**:
   - If no `type` key, add `type: "object"`
   - If has `properties` but `type` is not "object", set to "object"

8. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Empty/null schema returns `{"type": "object", "properties": {}}`
- [ ] Simple `$ref` references are inlined correctly
- [ ] Nested `$ref` chains are resolved recursively
- [ ] `$defs` is removed from final output
- [ ] Circular `$ref` is detected and returns `AdapterError`
- [ ] Depth exceeding 32 is detected and returns `AdapterError`
- [ ] Diamond-shaped references (non-circular) work correctly
- [ ] Missing `$ref` target returns error
- [ ] Unsupported `$ref` format returns error
- [ ] Root schema always has `type: "object"`
- [ ] Original schema is not mutated (clone-based)
- [ ] All tests pass, clippy clean

## Dependencies

- adapter-setup

## Estimated Time

3 hours
