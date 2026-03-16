# Feature: MCP Helpers

## Module Purpose
Provides MCP-specific helper functions for progress reporting and elicitation within module execution contexts.

## Public API Surface

### Types
- `ElicitAction` — enum: Accept, Decline, Cancel
- `ElicitResult` — struct: action (ElicitAction), content (Option<Value>)

### Functions
- `async report_progress(context, progress, total, message)`
- `async elicit(context, message, requested_schema) -> Option<ElicitResult>`

### Constants
- `MCP_PROGRESS_KEY: &str = "_mcp_progress"`
- `MCP_ELICIT_KEY: &str = "_mcp_elicit"`

## Acceptance Criteria
- [ ] report_progress reads progress callback from context data[MCP_PROGRESS_KEY]
- [ ] report_progress sends MCP notifications/progress with progress token
- [ ] elicit reads elicit callback from context data[MCP_ELICIT_KEY]
- [ ] elicit sends elicitation request via MCP session
- [ ] elicit returns None if no elicit callback available
- [ ] ElicitResult correctly maps action and optional content
