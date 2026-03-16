# Task: implement-stdio-transport

## Goal

Implement `run_stdio()` to read/write MCP JSON-RPC messages over stdin/stdout using tokio async I/O. This transport has no HTTP endpoints.

## Files Involved

- `src/server/transport.rs` — implement `run_stdio`

## Steps (TDD-first)

1. **Write tests first:**
   - Given mock `AsyncRead` + `AsyncWrite`, `run_stdio_with_io()` reads a JSON-RPC request line and delegates to the MCP server handler.
   - EOF on stdin causes graceful return (not panic).
   - Invalid JSON lines are logged and skipped.
2. **Define an `McpHandler` trait** (or use an existing server trait) that `TransportManager` accepts:
   ```rust
   #[async_trait::async_trait]
   pub trait McpHandler: Send + Sync {
       async fn handle_message(&self, message: serde_json::Value) -> Option<serde_json::Value>;
   }
   ```
   _Note: If an MCP handler trait already exists in the codebase (e.g., from the router module), use that instead._
3. **Implement `run_stdio()`:**
   ```rust
   pub async fn run_stdio(&self, handler: &dyn McpHandler) -> Result<(), TransportError> {
       self.run_stdio_with_io(tokio::io::stdin(), tokio::io::stdout(), handler).await
   }
   ```
4. **Implement `run_stdio_with_io()` (testable core):**
   - Wrap reader in `BufReader`.
   - Loop: `read_line()` until EOF.
   - Parse each line as `serde_json::Value`.
   - Call `handler.handle_message(msg)`.
   - If response is `Some`, serialize to JSON + newline, write to stdout, flush.
   - On parse error, log warning and continue.
   - On EOF, log info and return `Ok(())`.
5. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] Reads line-delimited JSON-RPC from stdin (or mock reader)
- [ ] Writes JSON-RPC responses to stdout (or mock writer) with newline delimiter
- [ ] EOF on stdin causes clean shutdown (returns Ok)
- [ ] Invalid JSON lines are logged and skipped (not fatal)
- [ ] I/O is abstracted behind `AsyncRead`/`AsyncWrite` for testability
- [ ] `run_stdio()` is the public entry point wrapping `run_stdio_with_io()`
- [ ] Tests pass with in-memory buffers

## Dependencies

- implement-health-metrics

## Estimated Time

1.5 hours
