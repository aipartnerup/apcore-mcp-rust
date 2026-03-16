# Feature: Server Factory

## Module Purpose
Creates and configures MCP servers from apcore registries. Builds MCP tool definitions from apcore module descriptors, registers request handlers, and produces initialization options.

## Public API Surface

### MCPServerFactory
- `new() -> MCPServerFactory`
- `create_server(name, version) -> Server`
- `build_tool(descriptor) -> Tool`
- `build_tools(registry, tags, prefix) -> Vec<Tool>`
- `register_handlers(server, tools, router)`
- `register_resource_handlers(server, registry)`
- `build_init_options(server, name, version) -> InitializationOptions`

## Acceptance Criteria
- [ ] Creates an MCP Server with correct name and version
- [ ] Converts apcore module descriptors to MCP Tool definitions
- [ ] Tool name uses module_id directly (dot-notation)
- [ ] Tool description includes AI intent metadata (x-when-to-use, x-when-not-to-use, etc.)
- [ ] Tool annotations map from apcore annotations (readOnlyHint, destructiveHint, etc.)
- [ ] Registers call_tool handler that delegates to ExecutionRouter
- [ ] Registers list_tools handler that returns all tools
- [ ] Registers resource handlers for modules with documentation
- [ ] Filters tools by tags when tags are specified
- [ ] Applies prefix to tool names when prefix is specified
