# Task: Build Initialization Options

## Summary

Implement `MCPServerFactory::build_init_options()` which constructs the MCP `InitializationOptions` containing the server name, version, and capabilities. This is the final configuration step before the server can be started.

## Approach (TDD-first)

### Tests to write first

1. **test_init_options_has_server_name** — Options include the provided server name.
2. **test_init_options_has_server_version** — Options include the provided version.
3. **test_init_options_has_capabilities** — Options include a capabilities object.
4. **test_init_options_tools_changed_capability** — Capabilities include `tools_changed: true` notification option.
5. **test_init_options_default_values** — Default name is `"apcore-mcp"`, default version is `"0.1.0"`.

### Implementation steps

1. Define `InitializationOptions` struct (if not already in mcp-types task):
   ```rust
   pub struct InitializationOptions {
       pub server_name: String,
       pub server_version: String,
       pub capabilities: ServerCapabilities,
   }

   pub struct ServerCapabilities {
       pub tools: Option<ToolsCapability>,
       pub resources: Option<ResourcesCapability>,
       pub experimental: HashMap<String, Value>,
   }

   pub struct ToolsCapability {
       pub list_changed: bool,
   }
   ```

2. Implement `build_init_options(&self, server: &MCPServer, name: &str, version: &str) -> InitializationOptions`:
   - Build capabilities from server state (tools registered implies tools capability, resources registered implies resources capability).
   - Set `tools_changed: true` in notification options.
   - Return fully constructed options.

3. Consider providing a builder pattern or default values for name/version.

## Files to modify

- Edit: `src/server/factory.rs`
- Edit: `src/server/types.rs` (add capability types if needed)

## Estimate

~1h

## Dependencies

- register-handlers
- register-resources
