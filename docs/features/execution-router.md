# Feature: Execution Router

## Module Purpose
Routes MCP tool call requests to the apcore executor, handling input validation, output formatting, error mapping, progress reporting, and streaming.

## Public API Surface

### ExecutionRouter
- `new(executor, validate_inputs, output_formatter) -> ExecutionRouter`
- `async handle_call(tool_name, arguments, extra) -> (Vec<ContentItem>, bool, Option<String>)`

## Acceptance Criteria
- [ ] Routes tool calls to the apcore executor by module_id
- [ ] Maps MCP arguments to apcore execution context
- [ ] Handles progress reporting via MCP notifications when progress_token is present
- [ ] Activates streaming path when executor supports stream() and progress_token + send_notification present
- [ ] Formats output using output_formatter (default: JSON)
- [ ] Maps apcore errors to MCP error responses via ErrorMapper
- [ ] Validates inputs when validate_inputs is true
- [ ] Returns (content_list, is_error, trace_id) tuple
- [ ] Passes identity from auth context to executor
- [ ] Supports elicitation via MCP session
