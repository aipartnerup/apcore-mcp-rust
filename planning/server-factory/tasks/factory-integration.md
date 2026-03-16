# Task: End-to-End Factory Integration

## Summary

Wire all factory components together into a cohesive integration test that exercises the full lifecycle: create factory, build tools from a mock registry, register handlers, register resource handlers, and build init options. Verify the complete data flow end-to-end.

## Approach (TDD-first)

### Tests to write first

1. **test_full_lifecycle** — Create factory, populate mock registry with 2 modules (one with docs, one without), build tools, register all handlers, build init options, verify everything is wired correctly.
2. **test_create_server_returns_server** — `create_server("test", "1.0.0")` returns a valid `MCPServer` with correct name.
3. **test_factory_new_initializes_components** — `MCPServerFactory::new()` creates instances of SchemaConverter and AnnotationMapper.
4. **test_end_to_end_tool_call** — Build tools from registry, register handlers, simulate a call_tool invocation through the handler, verify router is called with correct arguments.
5. **test_end_to_end_resource_read** — Register resource handlers, simulate read_resource for a documented module, verify documentation text is returned.

### Implementation steps

1. Update `MCPServerFactory::new()` to initialize internal components:
   ```rust
   impl MCPServerFactory {
       pub fn new() -> Self {
           Self {
               schema_converter: SchemaConverter,
               annotation_mapper: AnnotationMapper,
           }
       }
   }
   ```

2. Update `create_server()` signature to accept name and version:
   ```rust
   pub fn create_server(&self, name: &str, version: &str) -> MCPServer
   ```

3. Create integration test module `tests/server_factory_integration.rs` or inline in `src/server/factory.rs` as `#[cfg(test)] mod tests`.

4. Create mock implementations:
   - `MockModule` implementing `Module` trait with configurable description and schemas.
   - Populate `Registry` with mock modules and descriptors.
   - `MockExecutionRouter` or use the real `ExecutionRouter` with mock executor.

5. Verify the full pipeline:
   - Factory creates server with correct name.
   - Tools are built with correct names, descriptions, schemas, annotations.
   - Handlers respond correctly to list_tools and call_tool.
   - Resource handlers list and read documentation.
   - Init options contain correct server info and capabilities.

6. Update `MCPServerFactory` public API to match the feature spec exactly:
   - `new() -> MCPServerFactory`
   - `create_server(name, version) -> MCPServer`
   - `build_tool(descriptor) -> Tool`
   - `build_tools(registry, tags, prefix) -> Vec<Tool>`
   - `register_handlers(server, tools, router)`
   - `register_resource_handlers(server, registry)`
   - `build_init_options(server, name, version) -> InitializationOptions`

## Files to modify

- Edit: `src/server/factory.rs` (finalize public API, add integration tests)
- Edit: `src/server/server.rs` (ensure handler invocation works)

## Estimate

~3h

## Dependencies

- init-options (which transitively depends on all prior tasks)
