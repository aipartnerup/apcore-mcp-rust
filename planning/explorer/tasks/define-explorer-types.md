# Task: define-explorer-types

## Goal

Define the core types for the explorer module: `ExplorerConfig` (configuration struct), `ToolInfo` (tool metadata for display), and `HandleCallFn` (callback type for tool execution).

## Files Involved

- `src/explorer/mount.rs` — Add type definitions

## Steps (TDD-first)

1. **Write tests first** for `ExplorerConfig`:
   - Test that `ExplorerConfig::default()` sets `explorer_prefix` to `"/explorer"`, `allow_execute` to `false`, `title` to `"MCP Tool Explorer"`, and `authenticator` to `None`.
   - Test that builder methods override defaults.

2. **Write tests for `ToolInfo`**:
   - Test that `ToolInfo` serializes to JSON with `name`, `description`, and `inputSchema` fields.
   - Test round-trip serialization/deserialization.

3. **Define `ToolInfo` struct:**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ToolInfo {
       pub name: String,
       pub description: String,
       #[serde(rename = "inputSchema")]
       pub input_schema: serde_json::Value,
   }
   ```

4. **Define `HandleCallFn` type alias:**
   ```rust
   pub type HandleCallFn = Arc<
       dyn Fn(String, serde_json::Value) -> Pin<Box<dyn Future<Output = CallResult> + Send>>
           + Send
           + Sync,
   >;

   pub type CallResult = (Vec<serde_json::Value>, bool, Option<String>);
   ```

5. **Define `ExplorerConfig` struct:**
   ```rust
   pub struct ExplorerConfig {
       pub tools: Vec<ToolInfo>,
       pub handle_call: Option<HandleCallFn>,
       pub allow_execute: bool,
       pub explorer_prefix: String,
       pub authenticator: Option<Arc<dyn Authenticator>>,
       pub title: String,
       pub project_name: Option<String>,
       pub project_url: Option<String>,
   }
   ```

6. **Implement `ExplorerConfig::new()` and builder methods** for ergonomic construction.

7. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `ToolInfo` implements `Serialize` and `Deserialize` with correct field names
- [ ] `HandleCallFn` type alias compiles and is `Send + Sync`
- [ ] `ExplorerConfig` has all fields matching the Python `create_explorer_mount` signature
- [ ] Default values match Python defaults (prefix="/explorer", allow_execute=false, title="MCP Tool Explorer")
- [ ] Builder methods allow overriding all optional fields
- [ ] Tests pass for defaults and serialization
- [ ] `cargo check` passes

## Dependencies

None.

## Estimated Time

45 minutes
