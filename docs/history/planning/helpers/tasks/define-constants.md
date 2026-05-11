# Task: Define Constants

## Goal

Define the well-known string constants used as keys to store and retrieve MCP callbacks from the execution context's data map.

## Files Involved

- `src/helpers.rs` — add constants alongside existing type definitions

## Steps (TDD-first)

1. **Write tests first**: Add a test that asserts `MCP_PROGRESS_KEY == "_mcp_progress"` and `MCP_ELICIT_KEY == "_mcp_elicit"` to guard against accidental changes.
2. **Define constants**:
   ```rust
   /// Key for the progress-reporting callback in context data.
   pub const MCP_PROGRESS_KEY: &str = "_mcp_progress";

   /// Key for the elicitation callback in context data.
   pub const MCP_ELICIT_KEY: &str = "_mcp_elicit";
   ```
3. **Run tests** — confirm constant value tests pass.

## Acceptance Criteria

- [ ] `MCP_PROGRESS_KEY` equals `"_mcp_progress"` and is `pub`
- [ ] `MCP_ELICIT_KEY` equals `"_mcp_elicit"` and is `pub`
- [ ] Both constants have rustdoc comments
- [ ] Tests assert exact string values

## Dependencies

None

## Estimated Time

10 minutes
