# Task: Implement build_tools with Tag/Prefix Filtering

## Summary

Implement `MCPServerFactory::build_tools()` which iterates over a `Registry`, filters by tags and prefix, and produces a `Vec<Tool>` by calling `build_tool()` for each matching module. Modules without definitions or that fail conversion are logged and skipped.

## Approach (TDD-first)

### Tests to write first

1. **test_build_tools_all_modules** — Registry with 3 modules produces 3 tools.
2. **test_build_tools_tag_filter** — Only modules with matching tags are included.
3. **test_build_tools_prefix_filter** — Only modules with matching prefix are included.
4. **test_build_tools_no_definition_skipped** — Module in registry without descriptor is skipped with warning log.
5. **test_build_tools_build_error_skipped** — Module that fails `build_tool()` is skipped with warning log.
6. **test_build_tools_empty_registry** — Empty registry produces empty vec.
7. **test_build_tools_combined_filters** — Both tags and prefix applied simultaneously.

### Implementation steps

1. Implement `build_tools(&self, registry: &Registry, tags: Option<&[&str]>, prefix: Option<&str>) -> Vec<Tool>`:
   - Call `registry.list(tags, prefix)` to get filtered module IDs.
   - For each ID, call `registry.get_definition(id)`.
   - Skip `None` definitions (log warning).
   - Call `self.build_tool(descriptor, description)` wrapped in a match/result.
   - Skip errors (log warning).
   - Collect successful tools into `Vec<Tool>`.
2. Use `tracing::warn!` for skip logging (consistent with project's tracing dependency).

### Registry interaction

The `Registry::list()` method signature is:
```rust
pub fn list(&self, tags: Option<&[&str]>, prefix: Option<&str>) -> Vec<&str>
```

This already handles tag and prefix filtering, so `build_tools` delegates filtering entirely to the registry.

For the description, use `registry.describe(module_id)` which calls `module.description()`.

## Files to modify

- Edit: `src/server/factory.rs`

## Estimate

~2h

## Dependencies

- build-tool
