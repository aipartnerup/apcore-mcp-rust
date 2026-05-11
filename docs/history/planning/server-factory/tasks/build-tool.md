# Task: Implement build_tool from Descriptor

## Summary

Implement `MCPServerFactory::build_tool()` which converts a single `ModuleDescriptor` into an MCP `Tool`. This is the core conversion logic: schema conversion, annotation mapping, and tool construction.

## Approach (TDD-first)

### Tests to write first

1. **test_build_tool_name_is_module_name** — `descriptor.name` becomes `Tool.name` (dot-notation module_id).
2. **test_build_tool_description** — `descriptor` description (from registry `describe()` or a description field) is set on `Tool.description`.
3. **test_build_tool_input_schema** — `SchemaConverter::convert_input_schema` output is used as `Tool.inputSchema`.
4. **test_build_tool_annotations_mapped** — `ModuleAnnotations` on descriptor are mapped to `Tool.annotations` via `AnnotationMapper`.
5. **test_build_tool_meta_requires_approval** — When `requires_approval: true`, `Tool._meta` contains `{"requiresApproval": true}`.
6. **test_build_tool_meta_streaming** — When `streaming: true`, `Tool._meta` contains `{"streaming": true}`.
7. **test_build_tool_meta_both** — Both flags set produces `{"requiresApproval": true, "streaming": true}`.
8. **test_build_tool_meta_none** — Neither flag set produces `Tool._meta = None`.

### Implementation steps

1. Add `SchemaConverter`, `AnnotationMapper` as fields of `MCPServerFactory`.
2. Implement `build_tool(&self, descriptor: &ModuleDescriptor, description: &str) -> Result<Tool, FactoryError>`:
   - Call `SchemaConverter::convert_input_schema(&descriptor.input_schema)` for inputSchema.
   - Call `AnnotationMapper::to_mcp_annotations(&descriptor.annotations)` for annotations.
   - Check `AnnotationMapper::has_requires_approval()` and `is_streaming()` for `_meta`.
   - Construct and return `Tool`.
3. Define `FactoryError` enum (or reuse existing error type) for build failures.

### Key design decisions

- The Python code uses `descriptor.module_id` which maps to `ModuleDescriptor.name` in the Rust apcore crate.
- The Python code calls `SchemaExporter.export_mcp()` for annotation hints. In Rust, the `SchemaExporter::export_mcp()` does NOT include annotations. Instead, use `AnnotationMapper::to_mcp_annotations()` directly (the TS approach, which produces identical output per Python comment).
- Description enrichment with AI intent is handled in a separate task.

## Files to modify

- Edit: `src/server/factory.rs`

## Estimate

~4h

## Dependencies

- mcp-types
- tool-annotations-type
