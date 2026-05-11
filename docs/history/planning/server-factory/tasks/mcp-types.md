# Task: Define MCP Tool/Content Types

## Summary

Define the MCP protocol types needed by the server factory: `Tool`, `ToolAnnotations`, `TextContent`, `Resource`, `ReadResourceContents`, `InitializationOptions`, and `CallToolResult`. These are local Rust structs that mirror the MCP protocol and can later be replaced by an SDK crate.

## Approach (TDD-first)

### Tests to write first

1. **test_tool_serializes_to_mcp_json** — A `Tool` struct serializes to JSON with fields `name`, `description`, `inputSchema`, `annotations`, `_meta`.
2. **test_tool_annotations_defaults** — `ToolAnnotations::default()` produces `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: false`, `openWorldHint: true`, `title: None`.
3. **test_text_content_type_field** — `TextContent` serializes with `type: "text"` and `text: <value>`.
4. **test_resource_uri_format** — `Resource` includes `uri`, `name`, and `mimeType` fields.
5. **test_init_options_serialization** — `InitializationOptions` serializes with `server_name`, `server_version`, `capabilities`.

### Implementation steps

1. Create `src/server/types.rs` module.
2. Define `Tool`, `ToolAnnotations`, `TextContent`, `Resource`, `ReadResourceContents`, `CallToolResult`, `InitializationOptions` as `#[derive(Debug, Clone, Serialize, Deserialize)]` structs.
3. Use `#[serde(rename_all = "camelCase")]` where MCP uses camelCase.
4. Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields (`_meta`, `annotations`, `title`).
5. Add `mod types;` to `src/server/mod.rs`.

### Type definitions (key fields)

```rust
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

pub struct ToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "readOnlyHint")]
    pub read_only_hint: Option<bool>,
    #[serde(rename = "destructiveHint")]
    pub destructive_hint: Option<bool>,
    #[serde(rename = "idempotentHint")]
    pub idempotent_hint: Option<bool>,
    #[serde(rename = "openWorldHint")]
    pub open_world_hint: Option<bool>,
}

pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String, // always "text"
    pub text: String,
}

pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}
```

## Files to modify

- Create: `src/server/types.rs`
- Edit: `src/server/mod.rs` (add `pub mod types;`)

## Estimate

~2h

## Dependencies

None
