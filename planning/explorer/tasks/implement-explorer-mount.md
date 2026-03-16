# Task: implement-explorer-mount

## Goal

Implement `create_explorer_mount` which assembles the axum `Router` from the HTML template, API handlers, and configuration. This is the public entry point that replaces the current `todo!()` stub in `mount.rs`.

## Files Involved

- `src/explorer/mount.rs` — replace stub with full implementation

## Steps (TDD-first)

1. **Write a test** that calls `create_explorer_mount` with default config and two tools, then sends `GET /` and asserts 200 with HTML content-type.

2. **Write a test** that sends `GET /tools` and asserts 200 with JSON content-type and correct tool count.

3. **Write a test** that sends `POST /tools/my_tool/call` with `allow_execute=false` and asserts 403.

4. **Write a test** that sends `POST /tools/my_tool/call` with `allow_execute=true` and a mock handle_call, asserts 200 with expected result.

5. **Update `create_explorer_mount` signature:**
   ```rust
   pub fn create_explorer_mount(config: ExplorerConfig) -> Router {
   ```

6. **Build the Router:**
   ```rust
   let state = ExplorerState {
       tools: Arc::new(config.tools),
       handle_call: config.handle_call,
       allow_execute: config.allow_execute,
       authenticator: config.authenticator,
   };

   let html = render_html(&config.title, config.project_name.as_deref(), config.project_url.as_deref(), config.allow_execute);

   Router::new()
       .route("/", get(move || async move {
           ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
       }))
       .route("/tools", get(api::list_tools))
       .route("/tools/:name/call", post(api::call_tool))
       .with_state(state)
   ```

7. **Remove `#![allow(unused)]`** from mount.rs.

8. **Remove the `todo!()` macro.**

9. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `create_explorer_mount` accepts `ExplorerConfig` and returns `Router`
- [ ] `GET /` serves the HTML page with correct content-type
- [ ] `GET /tools` serves the JSON tool list
- [ ] `POST /tools/:name/call` is wired to the call handler
- [ ] HTML page title matches config
- [ ] `allow_execute` flag is respected in both HTML and API
- [ ] All `todo!()` macros removed
- [ ] `#![allow(unused)]` removed
- [ ] Tests pass
- [ ] `cargo check` passes

## Dependencies

- implement-api-handlers
- implement-html-template

## Estimated Time

1.5 hours
