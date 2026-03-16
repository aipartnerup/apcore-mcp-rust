# Task: Register Resource Handlers for Documentation URIs

## Summary

Implement `MCPServerFactory::register_resource_handlers()` which exposes module documentation as MCP resources with `docs://{module_id}` URIs. Iterates the registry, collects modules with documentation, and registers `list_resources` and `read_resource` handlers.

## Approach (TDD-first)

### Tests to write first

1. **test_list_resources_returns_documented_modules** — Modules with documentation appear as resources; modules without do not.
2. **test_resource_uri_format** — Resource URIs follow `docs://{module_id}` pattern.
3. **test_resource_name_format** — Resource name is `"{module_id} documentation"`.
4. **test_resource_mime_type** — All resources have `mimeType: "text/plain"`.
5. **test_read_resource_returns_documentation** — Reading a valid `docs://` URI returns the documentation text.
6. **test_read_resource_unknown_uri_errors** — Reading a non-existent module URI returns an error.
7. **test_read_resource_wrong_scheme_errors** — URIs not starting with `docs://` return an error.
8. **test_no_documented_modules** — Empty docs map results in empty resource list.

### Implementation steps

1. In `register_resource_handlers`:
   - Iterate `registry.list(None, None)` to get all module IDs.
   - For each, call `registry.get_definition(id)`.
   - Check for `documentation` field (see note below).
   - Build `HashMap<String, String>` mapping module_id to documentation text.
   - Wrap in `Arc<HashMap<String, String>>`.

2. Register `list_resources` handler:
   - Clone Arc, iterate docs_map, produce `Vec<Resource>` with `uri: format!("docs://{}", id)`.

3. Register `read_resource` handler:
   - Parse URI string, extract module_id after `docs://` prefix.
   - Look up in docs_map.
   - Return `ReadResourceContents { content, mime_type: "text/plain" }`.
   - Return error for unknown URIs or wrong scheme.

4. Add handler storage to `MCPServer`:
   ```rust
   list_resources_handler: Option<Arc<dyn Fn() -> Vec<Resource> + Send + Sync>>,
   read_resource_handler: Option<Arc<dyn Fn(String) -> Result<Vec<ReadResourceContents>, FactoryError> + Send + Sync>>,
   ```

### Note on documentation field

The Rust `ModuleDescriptor` currently lacks a `documentation: Option<String>` field. This must be added to `ModuleDescriptor` in apcore-rust, or accessed via the `Module` trait (which also lacks it). Options:
- Add `pub documentation: Option<String>` to `ModuleDescriptor` with `#[serde(default)]`.
- Use `Module::description()` as a fallback (different semantics but functional).
- Document the gap and add the field as part of this task.

## Files to modify

- Edit: `src/server/factory.rs`
- Edit: `src/server/server.rs` (add resource handler storage)
- Possibly: `apcore-rust/src/registry/registry.rs` (add documentation field)

## Estimate

~3h

## Dependencies

- build-tools
