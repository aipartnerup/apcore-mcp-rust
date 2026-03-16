# Task: Register list_tools and call_tool Handlers

## Summary

Implement `MCPServerFactory::register_handlers()` which installs the `list_tools` and `call_tool` request handlers on the MCP server. In Python, these are closures capturing shared state. In Rust, shared state must use `Arc<Vec<Tool>>` and `Arc<ExecutionRouter>` for thread-safe sharing across async handler closures.

## Approach (TDD-first)

### Tests to write first

1. **test_list_tools_returns_all_tools** — After registration, calling the list_tools handler returns all tools.
2. **test_call_tool_delegates_to_router** — Calling call_tool with a tool name and arguments invokes `ExecutionRouter::handle_call`.
3. **test_call_tool_returns_text_content** — Successful execution returns `Vec<TextContent>`.
4. **test_call_tool_error_raises** — When router returns `is_error: true`, handler returns an error.
5. **test_call_tool_with_progress_token** — Progress token is forwarded to router via `extra`.
6. **test_call_tool_with_identity** — Auth identity is passed through to router.

### Implementation steps

1. Define a handler registration model on `MCPServer`. Options:
   - **Option A (closure-based):** `MCPServer` stores `Box<dyn Fn(...) -> ...>` handlers, matching Python's pattern.
   - **Option B (trait-based):** Define `ToolHandler` and `ResourceHandler` traits, register trait objects.
   - **Recommended: Option A** for closest parity. Store handler closures in `MCPServer` fields.

2. Add handler storage to `MCPServer`:
   ```rust
   pub struct MCPServer {
       // ...existing fields...
       list_tools_handler: Option<Arc<dyn Fn() -> Vec<Tool> + Send + Sync>>,
       call_tool_handler: Option<Arc<dyn Fn(String, Value, Option<Value>) -> Pin<Box<dyn Future<Output = CallToolResult> + Send>> + Send + Sync>>,
   }
   ```

3. Implement `register_handlers(&self, server: &mut MCPServer, tools: Vec<Tool>, router: Arc<ExecutionRouter>)`:
   - Wrap `tools` in `Arc<Vec<Tool>>`.
   - Clone `Arc<ExecutionRouter>` for the call_tool closure.
   - Create `list_tools` closure that returns `tools.clone()`.
   - Create `call_tool` closure that:
     a. Extracts progress token from extra/context.
     b. Extracts auth identity from context.
     c. Calls `router.handle_call(name, arguments, extra).await`.
     d. Maps result to `Vec<TextContent>` or error.
   - Set handlers on `server`.

4. Progress token and identity bridging:
   - In Python, these come from `request_ctx` context var and `auth_identity_var`.
   - In Rust, use tokio task-local variables or pass through the `extra: Option<&Value>` parameter.
   - Define `CallContext` struct to carry progress_token, session, identity.

### Key design decisions

- Handlers must be `Send + Sync + 'static` for async compatibility.
- The `ExecutionRouter` must be wrapped in `Arc` (not `Arc<Mutex>`) since `handle_call` takes `&self`.
- Error handling: when `is_error` is true, convert first text content to error (matching Python's `raise Exception(...)` pattern).

## Files to modify

- Edit: `src/server/factory.rs`
- Edit: `src/server/server.rs` (add handler storage)

## Estimate

~4h

## Dependencies

- build-tools
