# Task: convenience-functions

## Objective
Implement the three top-level convenience functions (`serve`, `async_serve`, `to_openai_tools`) that construct a temporary `APCoreMCP` internally.

## Estimate
~30 min

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convenience_serve_accepts_backend_source() {
        // Should construct APCoreMCP internally and delegate
        // Test that it accepts the same polymorphic inputs
    }

    #[test]
    fn convenience_to_openai_tools_with_tags() {
        // Verify tags/prefix flow through to the converter
    }

    #[test]
    fn convenience_async_serve_accepts_name() {
        // Verify name parameter flows through
    }

    #[test]
    fn convenience_serve_config_struct_has_defaults() {
        let cfg = ServeConfig::default();
        assert_eq!(cfg.transport, "stdio");
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 8000);
        assert_eq!(cfg.name, "apcore-mcp");
    }
}
```

## Implementation Steps
1. Define `ServeConfig` struct (or reuse `APCoreMCPConfig`) for convenience function parameters
2. Implement `pub fn serve(backend: impl Into<BackendSource>, config: ServeConfig) -> Result<(), APCoreMCPError>`:
   - Build `APCoreMCP` via builder with config fields
   - Call `.serve()`
3. Implement `pub async fn async_serve(backend: impl Into<BackendSource>, config: AsyncServeConfig) -> Result<..., APCoreMCPError>`:
   - Build `APCoreMCP` via builder
   - Call `.async_serve()`
4. Implement `pub fn to_openai_tools(backend: impl Into<BackendSource>, opts: OpenAIToolsConfig) -> Result<Vec<Value>, APCoreMCPError>`:
   - Build `APCoreMCP` via builder
   - Call `.to_openai_tools()`
5. Ensure all config structs implement `Default`

## Acceptance Criteria
- [ ] All three functions accept `impl Into<BackendSource>` for ergonomic usage
- [ ] Config structs have sensible defaults
- [ ] Functions are thin wrappers that delegate to `APCoreMCP` methods
- [ ] Error types propagate correctly

## Dependencies
- `serve-methods`
- `to-openai-tools`

## Files Modified
- `src/apcore_mcp.rs`
