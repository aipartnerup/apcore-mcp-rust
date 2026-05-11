# Task: convert-registry

## Goal

Implement `OpenAIConverter::convert_registry` that iterates over all modules in an apcore `Registry`, applies tag/prefix filtering, and converts each descriptor to an OpenAI tool definition.

## Files Involved

- `src/converters/openai.rs` — Implement `convert_registry` method

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_convert_registry_empty` — Empty registry returns empty vec
   - `test_convert_registry_single_module` — Registry with one module produces one tool definition
   - `test_convert_registry_multiple_modules` — Registry with multiple modules produces matching count
   - `test_convert_registry_skips_missing_descriptor` — Module ID in list but `get_definition` returns `None` is skipped (race condition)
   - `test_convert_registry_with_tags_filter` — Only modules matching tags are included
   - `test_convert_registry_with_prefix_filter` — Only modules matching prefix are included
   - `test_convert_registry_passes_embed_annotations` — `embed_annotations` flag is forwarded to `convert_descriptor`
   - `test_convert_registry_passes_strict` — `strict` flag is forwarded to `convert_descriptor`

2. **Define method signature**:
   ```rust
   pub fn convert_registry(
       &self,
       registry: &Registry,
       embed_annotations: bool,
       strict: bool,
       tags: Option<&[&str]>,
       prefix: Option<&str>,
   ) -> Result<Vec<Value>, ConverterError>
   ```

3. **Implement the method**:
   ```rust
   let module_ids = registry.list(tags, prefix);
   let mut tools = Vec::new();
   for module_id in module_ids {
       let descriptor = match registry.get_definition(module_id) {
           Some(d) => d,
           None => continue, // Race condition: module unregistered between list and get
       };
       // Get description from module trait
       let description = registry.get_module_description(module_id)
           .unwrap_or_default();
       let tool = self.convert_descriptor(
           module_id,
           &description,
           descriptor,
           embed_annotations,
           strict,
       )?;
       tools.push(tool);
   }
   Ok(tools)
   ```

4. **Verify Registry API**: Confirm how to get the module description from the registry. The `Registry` struct has `modules: HashMap<String, Box<dyn Module>>` — we may need to call `registry.get(module_id)` to get the `Module` reference and call `.description()` on it. If there is no public accessor, we may need to accept a description lookup closure or adjust the API.

5. **Run tests** — ensure all pass. Run `cargo clippy`.

## Implementation Notes

- The Python implementation uses `registry.list(tags=tags, prefix=prefix)` for filtering. The Rust `Registry::list()` accepts `Option<&[&str]>` for tags and `Option<&str>` for prefix — matching API.
- Must handle the case where `get_definition` returns `None` gracefully (skip, not error).
- If `Registry` does not expose a way to get module descriptions, consider two alternatives:
  (a) Accept a `HashMap<String, String>` of module_id -> description
  (b) Use `descriptor.name` as a fallback description

## Acceptance Criteria

- [ ] Iterates all modules from `registry.list(tags, prefix)`
- [ ] Skips modules where `get_definition` returns `None`
- [ ] Each module is converted via `convert_descriptor`
- [ ] Tag filtering works correctly
- [ ] Prefix filtering works correctly
- [ ] `embed_annotations` and `strict` flags are forwarded
- [ ] Returns `Result` with proper error propagation
- [ ] All tests pass, clippy clean

## Dependencies

- convert-descriptor

## Estimated Time

1 hour
