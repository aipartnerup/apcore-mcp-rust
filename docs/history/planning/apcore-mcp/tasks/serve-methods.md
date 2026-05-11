# Task: serve-methods

## Objective
Implement `serve()` (blocking) and `async_serve()` (async, returns embeddable app) on `APCoreMCP`.

## Estimate
~1.5 hr

## TDD Tests (write first)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serve_rejects_unknown_transport() {
        let mcp = make_test_apcore_mcp_with_transport("websocket");
        let result = mcp.serve();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown transport"));
    }

    #[test]
    fn serve_rejects_invalid_explorer_prefix() {
        let mcp = make_test_apcore_mcp();
        let result = mcp.serve_with_options(ServeOptions {
            explorer: true,
            explorer_prefix: "no-slash".into(),
            ..Default::default()
        });
        assert!(matches!(result, Err(APCoreMCPError::InvalidExplorerPrefix)));
    }

    #[tokio::test]
    async fn async_serve_returns_router() {
        let mcp = make_test_apcore_mcp();
        let result = mcp.async_serve(AsyncServeOptions::default()).await;
        assert!(result.is_ok());
        // The returned value should be an axum Router or app handle
    }

    #[test]
    fn serve_calls_on_startup_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let mcp = make_test_apcore_mcp();
        // serve with on_startup that sets flag
        // (will need to cancel quickly or use a test transport)
    }

    #[test]
    fn serve_calls_on_shutdown_callback_even_on_error() {
        // Verify finally-like behavior for on_shutdown
    }
}
```

## Implementation Steps
1. Define `ServeOptions` struct for serve-time parameters (transport, host, port, explorer settings, callbacks)
2. Implement `serve(&self)` with default options and `serve_with_options(&self, opts: ServeOptions)`:
   - Validate explorer_prefix if explorer enabled
   - Call `build_server_components()`
   - Build explorer routes if enabled and HTTP transport
   - Build auth middleware if authenticator present and HTTP transport
   - Create `TransportManager`, set module count
   - Match on transport: "stdio" | "streamable-http" | "sse" | unknown
   - Call `on_startup` before, `on_shutdown` in finally/drop guard
   - Use `tokio::runtime::Runtime::new()?.block_on(async_run)` for blocking
3. Implement `async_serve(&self)` / `async_serve_with_options(&self, opts: AsyncServeOptions)`:
   - Similar setup but returns the axum `Router` via a guard struct
   - Guard implements `Drop` for cleanup
4. Create `ServeGuard` struct that holds the router and cleanup state

## Acceptance Criteria
- [ ] `serve()` blocks and runs the transport
- [ ] `async_serve()` returns an app handle for embedding
- [ ] Explorer routes added only for HTTP transports
- [ ] Auth middleware added only for HTTP transports with authenticator
- [ ] on_startup/on_shutdown callbacks invoked correctly
- [ ] Unknown transport produces clear error

## Dependencies
- `build-server-components`
- Depends on `TransportManager` being at least stub-complete

## Files Modified
- `src/apcore_mcp.rs`
