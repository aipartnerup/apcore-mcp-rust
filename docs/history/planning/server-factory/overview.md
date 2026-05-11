# Server Factory — Overview

## Feature

Port the Python `MCPServerFactory` to Rust, providing the adapter surface that converts apcore registries into MCP server configurations.

## Scope

The server factory is the central orchestration point for MCP server construction. It:

1. **Converts** apcore `ModuleDescriptor` + `ModuleAnnotations` into MCP `Tool` definitions with proper schema, annotations, AI intent metadata, and `_meta` flags.
2. **Registers** request handlers (`list_tools`, `call_tool`, `list_resources`, `read_resource`) on the MCP server using `Arc`-shared state for thread-safe async access.
3. **Produces** `InitializationOptions` for server startup.

## Reference Implementation

Python: `apcore-mcp-python/src/apcore_mcp/server/factory.py` (288 lines, HIGH complexity)

Key behaviors:
- Tool name = `ModuleDescriptor.name` (dot-notation)
- Annotations mapped via `SchemaExporter.export_mcp()` in Python, but `AnnotationMapper` directly in Rust (equivalent output)
- AI intent keys (`x-when-to-use` etc.) appended to tool description
- `_meta` carries `requiresApproval` and `streaming` flags
- Resource handlers expose `docs://{module_id}` URIs for module documentation
- `call_tool` handler bridges progress tokens and auth identity to `ExecutionRouter`

## Task Summary

| Task | Description | Estimate | Dependencies |
|------|-------------|----------|--------------|
| mcp-types | Define MCP protocol types (Tool, ToolAnnotations, TextContent, Resource, etc.) | ~2h | - |
| tool-annotations-type | Implement AnnotationMapper: ModuleAnnotations to ToolAnnotations | ~2h | - |
| build-tool | Core build_tool conversion logic | ~4h | mcp-types, tool-annotations-type |
| ai-intent-metadata | AI intent key extraction and description enrichment | ~2h | build-tool |
| build-tools | Iterate registry with tag/prefix filtering | ~2h | build-tool |
| register-handlers | Install list_tools and call_tool handlers with Arc-shared state | ~4h | build-tools |
| register-resources | Install list_resources and read_resource handlers for docs:// URIs | ~3h | build-tools |
| init-options | Build InitializationOptions with capabilities | ~1h | register-handlers, register-resources |
| factory-integration | End-to-end integration wiring and tests | ~3h | init-options |

**Total estimate: ~23h**

## Execution Order

1. mcp-types, tool-annotations-type (parallel)
2. build-tool
3. ai-intent-metadata, build-tools (parallel)
4. register-handlers, register-resources (parallel)
5. init-options
6. factory-integration

## Key Design Decisions

- **Local MCP types**: Define Rust structs mirroring MCP protocol (no MCP SDK crate available). These can be swapped later.
- **Arc over Mutex**: Handlers share tools list and router via `Arc`, avoiding `Mutex` since handler state is read-only after registration.
- **AnnotationMapper over SchemaExporter**: Use `AnnotationMapper::to_mcp_annotations()` directly (TypeScript approach) rather than routing through `SchemaExporter.export_mcp()`. The Python comment confirms both paths produce identical output.
- **Typed descriptors**: Accept `&ModuleDescriptor` and `&ModuleAnnotations` (from apcore crate) instead of `&Value` for compile-time safety.
- **Missing fields**: `ModuleDescriptor` in apcore-rust needs `documentation: Option<String>` and `metadata: Option<HashMap<String, Value>>` fields added.

## Files

- Plan: `planning/server-factory/plan.md`
- Tasks: `planning/server-factory/tasks/*.md`
- Implementation target: `src/server/factory.rs`
- Supporting types: `src/server/types.rs` (new)
- Adapter updates: `src/adapters/annotations.rs`
- Server updates: `src/server/server.rs`
