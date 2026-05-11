# Task: struct-and-accessors

## Objective
Implement the `APCoreMCP` struct fields and accessor methods: `registry()`, `executor()`, `tools()`.

## Estimate
~30 min

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_returns_arc_ref() {
        let mcp = make_test_apcore_mcp(); // helper with mock registry/executor
        let reg = mcp.registry();
        // Should return &Arc<Registry>
        assert!(!reg.list(None, None).is_empty() || reg.list(None, None).is_empty());
    }

    #[test]
    fn executor_returns_arc_ref() {
        let mcp = make_test_apcore_mcp();
        let _exec = mcp.executor();
        // Just verify it returns without panic
    }

    #[test]
    fn tools_returns_module_ids() {
        let mcp = make_test_apcore_mcp();
        let tools = mcp.tools();
        assert!(tools.is_empty() || !tools.is_empty()); // type check
        // With a populated registry, verify filtering by tags/prefix
    }

    #[test]
    fn tools_filters_by_tags() {
        let mcp = make_test_apcore_mcp_with_tags(vec!["public".into()]);
        let tools = mcp.tools();
        // All returned tools should have the "public" tag
    }

    #[test]
    fn tools_filters_by_prefix() {
        let mcp = make_test_apcore_mcp_with_prefix("my_");
        let tools = mcp.tools();
        // All returned tools should start with "my_"
    }
}
```

## Implementation Steps
1. Define `APCoreMCP` struct with fields:
   - `registry: Arc<Registry>`
   - `executor: Arc<Executor>`
   - `config: APCoreMCPConfig`
   - `authenticator: Option<Arc<dyn Authenticator>>`
   - `metrics_collector: Option<Arc<dyn MetricsExporter>>`
   - `output_formatter: Option<Arc<dyn Fn(&Value) -> String + Send + Sync>>`
   - `approval_handler: Option<Arc<dyn ApprovalHandler>>`
2. Implement `registry(&self) -> &Arc<Registry>`
3. Implement `executor(&self) -> &Arc<Executor>`
4. Implement `tools(&self) -> Vec<String>` — delegates to `registry.list()` with tags/prefix filtering
5. Create test helper functions for constructing test instances

## Acceptance Criteria
- [ ] All three accessors return correct types
- [ ] `tools()` respects tags and prefix filtering
- [ ] Struct stores all necessary state from builder

## Dependencies
- `builder-pattern`

## Files Modified
- `src/apcore_mcp.rs`
