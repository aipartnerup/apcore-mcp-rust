# Task: implement-api-handlers

## Goal

Implement the JSON API handlers that power the explorer: `GET /tools` returns tool metadata as JSON, and `POST /tools/:name/call` executes a tool and returns the result. These handlers are standalone async functions that will be wired into the axum Router in the next task.

## Files Involved

- `src/explorer/api.rs` — new file with handler functions
- `src/explorer/mod.rs` — add `pub mod api;`

## Steps (TDD-first)

1. **Define `ExplorerState`** (shared state for handlers):
   ```rust
   #[derive(Clone)]
   pub struct ExplorerState {
       pub tools: Arc<Vec<ToolInfo>>,
       pub handle_call: Option<HandleCallFn>,
       pub allow_execute: bool,
       pub authenticator: Option<Arc<dyn Authenticator>>,
   }
   ```

2. **Write a test for `list_tools`:**
   - Create an `ExplorerState` with two `ToolInfo` entries.
   - Call the handler, assert 200 with a JSON array of two objects.
   - Assert each object has `name`, `description`, `inputSchema`.

3. **Implement `list_tools` handler:**
   ```rust
   pub async fn list_tools(
       State(state): State<ExplorerState>,
   ) -> impl IntoResponse {
       Json(&*state.tools)
   }
   ```

4. **Write tests for `call_tool`:**
   - Test with `allow_execute=false` -> 403 Forbidden JSON response.
   - Test with `allow_execute=true`, no authenticator, valid tool name -> mock handle_call returns result, assert 200 JSON.
   - Test with authenticator set, no Bearer token -> 401.
   - Test with authenticator set, valid Bearer token -> executes with AUTH_IDENTITY set.
   - Test with unknown tool name -> 404.

5. **Implement `call_tool` handler:**
   ```rust
   pub async fn call_tool(
       State(state): State<ExplorerState>,
       Path(tool_name): Path<String>,
       headers: HeaderMap,
       Json(args): Json<serde_json::Value>,
   ) -> impl IntoResponse { ... }
   ```
   - Check `allow_execute`, return 403 if false.
   - Validate tool name exists in `state.tools`, return 404 if not.
   - If authenticator is set, extract Bearer token from headers, authenticate, return 401 on failure.
   - Call `handle_call` with tool name and args.
   - If authenticator provided, wrap call in `AUTH_IDENTITY.scope(Some(identity), ...)`.
   - Return JSON response with content items, is_error flag, and error code.

6. **Define response types:**
   ```rust
   #[derive(Serialize)]
   pub struct CallResponse {
       pub content: Vec<serde_json::Value>,
       pub is_error: bool,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub error_code: Option<String>,
   }
   ```

7. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `list_tools` returns 200 with JSON array of all tools
- [ ] `call_tool` returns 403 when `allow_execute=false`
- [ ] `call_tool` returns 404 for unknown tool names
- [ ] `call_tool` returns 401 when authenticator is set and auth fails
- [ ] `call_tool` bridges `AUTH_IDENTITY` task-local when authenticator is present
- [ ] `call_tool` returns JSON `CallResponse` on success
- [ ] All tests pass
- [ ] `cargo check` passes

## Dependencies

- define-explorer-types

## Estimated Time

1.5 hours
