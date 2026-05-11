# Task: add-transport-unit-tests

## Goal

Write comprehensive unit tests for all transport components: error types, health/metrics, host/port validation, stdio I/O, and router construction for HTTP transports.

## Files Involved

- `src/server/transport.rs` — add `#[cfg(test)] mod tests` block

## Steps (TDD-first)

1. **Create a mock `MetricsExporter`:**
   ```rust
   struct MockExporter(String);
   impl MetricsExporter for MockExporter {
       fn export_prometheus(&self) -> String { self.0.clone() }
   }
   ```
2. **Create a mock `McpHandler`:**
   ```rust
   struct EchoHandler;
   #[async_trait::async_trait]
   impl McpHandler for EchoHandler {
       async fn handle_message(&self, msg: Value) -> Option<Value> {
           Some(msg) // echo back
       }
   }
   ```
3. **TransportError tests:**
   - `InvalidHost` displays correctly.
   - `InvalidPort` displays correctly.
   - `Io` wraps `std::io::Error`.
4. **Health response tests:**
   - Default module_count is 0.
   - After `set_module_count(5)`, health response shows 5.
   - `uptime_seconds` is non-negative.
   - `status` is `"ok"`.
5. **Metrics response tests:**
   - With `None` exporter: 404 status.
   - With mock exporter: 200 status, correct content-type, body matches exporter output.
6. **Host/port validation tests:**
   - Empty host returns `InvalidHost`.
   - Port 0 returns `InvalidPort` (if validated).
   - Valid host/port returns Ok.
7. **Stdio transport tests (using in-memory I/O):**
   - Single JSON-RPC request -> echo response.
   - Empty input (immediate EOF) -> Ok return.
   - Invalid JSON line -> skipped, next valid line processed.
8. **HTTP router tests (using `tower::ServiceExt::oneshot`):**
   - GET /health returns 200 JSON.
   - GET /metrics returns 404 (no exporter).
   - GET /metrics returns 200 (with exporter).
9. **Run `cargo test`.**

## Acceptance Criteria

- [ ] All error variant display messages tested
- [ ] Health response fields tested
- [ ] Metrics with/without exporter tested
- [ ] Host/port validation edge cases tested
- [ ] Stdio I/O tested with in-memory buffers
- [ ] HTTP router tested via oneshot requests
- [ ] All tests pass with `cargo test`

## Dependencies

- implement-stdio-transport
- implement-streamable-http-transport
- implement-sse-transport

## Estimated Time

1.5 hours
