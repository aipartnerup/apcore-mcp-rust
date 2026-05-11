# Task: to-openai-tools

## Objective
Implement `to_openai_tools()` on `APCoreMCP` that delegates to `OpenAIConverter`.

## Estimate
~30 min

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_openai_tools_returns_vec_of_values() {
        let mcp = make_test_apcore_mcp();
        let tools = mcp.to_openai_tools(false, false);
        // Should return Vec<Value>, possibly empty for test registry
        assert!(tools.is_empty() || tools.iter().all(|t| t.is_object()));
    }

    #[test]
    fn to_openai_tools_passes_embed_annotations() {
        let mcp = make_test_apcore_mcp();
        let tools = mcp.to_openai_tools(true, false);
        // Verify annotations are embedded (depends on converter impl)
    }

    #[test]
    fn to_openai_tools_passes_strict_flag() {
        let mcp = make_test_apcore_mcp();
        let tools = mcp.to_openai_tools(false, true);
        // Verify strict: true is set in output (depends on converter impl)
    }

    #[test]
    fn to_openai_tools_respects_tags_and_prefix() {
        let mcp = make_test_apcore_mcp_with_tags(vec!["public".into()]);
        let tools = mcp.to_openai_tools(false, false);
        // Should only include tools matching the tags filter
    }
}
```

## Implementation Steps
1. Change `to_openai_tools` signature to accept `embed_annotations: bool` and `strict: bool`
2. Create `OpenAIConverter` instance
3. Call `converter.convert_registry(&self.registry, embed_annotations, strict, tags, prefix)`
4. Log the count at debug level via `tracing::debug!`
5. Return the `Vec<Value>`

## Acceptance Criteria
- [ ] Delegates correctly to `OpenAIConverter`
- [ ] Passes through all parameters (embed_annotations, strict, tags, prefix)
- [ ] Returns `Vec<Value>` matching OpenAI tool format

## Dependencies
- `struct-and-accessors`
- Depends on `OpenAIConverter` being at least stub-complete

## Files Modified
- `src/apcore_mcp.rs`
