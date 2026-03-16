# Task: build-server-components

## Objective
Implement the shared `build_server_components()` method that constructs the MCP server, tools list, execution router, and init options. This mirrors Python's `_build_server_components()`.

## Estimate
~45 min

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_server_components_returns_all_parts() {
        let mcp = make_test_apcore_mcp();
        let components = mcp.build_server_components();
        assert!(components.is_ok());
        let (server, router, tools, init_options, version) = components.unwrap();
        // Verify types and basic properties
        assert!(!version.is_empty());
    }

    #[test]
    fn build_server_components_uses_custom_version() {
        let mcp = make_test_apcore_mcp_with_version("2.0.0");
        let (_, _, _, _, version) = mcp.build_server_components().unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn build_server_components_defaults_to_crate_version() {
        let mcp = make_test_apcore_mcp(); // no version set
        let (_, _, _, _, version) = mcp.build_server_components().unwrap();
        assert_eq!(version, crate::VERSION);
    }

    #[test]
    fn build_server_components_applies_tag_filter() {
        let mcp = make_test_apcore_mcp_with_tags(vec!["api".into()]);
        let (_, _, tools, _, _) = mcp.build_server_components().unwrap();
        // tools should only include modules tagged "api"
    }
}
```

## Implementation Steps
1. Define return type (tuple or struct): `(MCPServer, ExecutionRouter, Vec<ToolDef>, InitOptions, String)`
2. Use `MCPServerFactory::new()` to create a factory
3. Call `factory.create_server(name, version)` to build the server
4. Call `factory.build_tools(registry, tags, prefix)` to get filtered tools
5. Create `ExecutionRouter` with executor, validate_inputs, and output_formatter
6. Call `factory.register_handlers(server, tools, router)`
7. Call `factory.register_resource_handlers(server, registry)`
8. Call `factory.build_init_options(server, name, version)`
9. Return the tuple
10. Default version to `crate::VERSION` when `config.version` is `None`

## Acceptance Criteria
- [ ] Returns all five components
- [ ] Version defaults to crate version
- [ ] Tag and prefix filters are passed through to factory
- [ ] Router receives validate_inputs and output_formatter settings

## Dependencies
- `struct-and-accessors`
- Depends on `MCPServerFactory`, `ExecutionRouter` being at least stub-complete

## Files Modified
- `src/apcore_mcp.rs`
