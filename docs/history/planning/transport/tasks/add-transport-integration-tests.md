# Task: add-transport-integration-tests

## Goal

Write integration tests that spin up actual HTTP servers on ephemeral ports and verify end-to-end behavior: health check round-trip, metrics endpoint, and MCP endpoint reachability.

## Files Involved

- `tests/transport_integration.rs` — new integration test file

## Steps (TDD-first)

1. **Helper: start server on port 0, return bound address:**
   ```rust
   async fn start_test_server(tm: Arc<TransportManager>, handler: Arc<dyn McpHandler>) -> (SocketAddr, JoinHandle<()>) {
       let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
       let addr = listener.local_addr().unwrap();
       let app = tm.build_streamable_http_app(handler, None);
       let handle = tokio::spawn(async move {
           axum::serve(listener, app).await.unwrap();
       });
       (addr, handle)
   }
   ```
2. **Test: health endpoint returns 200 with expected JSON:**
   - Start server.
   - `reqwest::get(format!("http://{}/health", addr))` (or use hyper client).
   - Assert status 200, body contains `"status": "ok"`, `"module_count"`.
   - Abort server handle.
3. **Test: metrics endpoint returns 404 without exporter:**
   - Start server with `TransportManager::new(None)`.
   - GET /metrics -> 404.
4. **Test: metrics endpoint returns 200 with exporter:**
   - Start server with mock exporter.
   - GET /metrics -> 200, body matches exporter output.
5. **Test: /mcp endpoint is reachable (returns non-404):**
   - POST /mcp with empty body -> some response (400 or valid JSON-RPC error).
6. **Test: extra routes are served:**
   - Add a custom route `/custom` that returns 200.
   - Start server with extra_routes.
   - GET /custom -> 200.
7. **Run `cargo test -- --test transport_integration`.**

## Acceptance Criteria

- [ ] Tests use ephemeral ports (port 0) to avoid conflicts
- [ ] Health endpoint returns correct JSON structure
- [ ] Metrics endpoint returns 404 or 200 based on exporter presence
- [ ] MCP endpoint is reachable at /mcp
- [ ] Extra routes are properly served
- [ ] Server handle is cleaned up after each test
- [ ] All tests pass in CI

## Dependencies

- add-transport-unit-tests

## Estimated Time

1.5 hours
