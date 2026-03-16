# Task: define-transport-kind-enum

## Goal

Define a `TransportKind` enum to replace the stringly-typed `transport` parameter in MCPServer. This provides compile-time exhaustiveness checking and eliminates the "Unknown transport" runtime error path.

## Files Involved

- `src/server/server.rs` — add `TransportKind` enum, update `MCPServer` to use it

## Steps (TDD-first)

1. **Write tests first:**
   - `TransportKind::from_str("stdio")` returns `Ok(TransportKind::Stdio)`.
   - `TransportKind::from_str("streamable-http")` returns `Ok(TransportKind::StreamableHttp)`.
   - `TransportKind::from_str("sse")` returns `Ok(TransportKind::Sse)`.
   - `TransportKind::from_str("STDIO")` returns `Ok(TransportKind::Stdio)` (case-insensitive).
   - `TransportKind::from_str("unknown")` returns `Err`.
   - `TransportKind::Stdio.address("127.0.0.1", 8000)` returns `"stdio"`.
   - `TransportKind::StreamableHttp.address("127.0.0.1", 8000)` returns `"http://127.0.0.1:8000"`.
   - `TransportKind::Sse.address("0.0.0.0", 9090)` returns `"http://0.0.0.0:9090"`.
2. **Define the enum:**
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum TransportKind {
       Stdio,
       StreamableHttp,
       Sse,
   }
   ```
3. **Implement `std::str::FromStr`** for `TransportKind` with case-insensitive matching.
4. **Implement `TransportKind::address(&self, host: &str, port: u16) -> String`** method:
   - `Stdio` returns `"stdio"`.
   - `StreamableHttp` and `Sse` return `"http://{host}:{port}"`.
5. **Implement `Display`** for `TransportKind` for logging.
6. **Update `MCPServer` field** from `transport: String` to `transport: TransportKind`.
7. **Update `MCPServer::address()`** to delegate to `TransportKind::address()`.
8. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] `TransportKind` has three variants: `Stdio`, `StreamableHttp`, `Sse`
- [ ] `FromStr` implementation is case-insensitive
- [ ] Invalid transport string returns an error (not panic)
- [ ] `address()` returns "stdio" for Stdio, "http://{host}:{port}" for HTTP variants
- [ ] `MCPServer` uses `TransportKind` instead of `String`
- [ ] All tests pass

## Dependencies

None

## Estimated Time

30 minutes
