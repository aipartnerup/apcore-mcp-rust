# Task: implement-streamable-http-transport

## Goal

Implement `run_streamable_http()` and `build_streamable_http_app()` to serve MCP over HTTP with the streamable-http protocol. The MCP endpoint is mounted at `/mcp`. Health and metrics endpoints are auto-registered.

## Files Involved

- `src/server/transport.rs` — implement both methods

## Steps (TDD-first)

1. **Write tests first:**
   - `build_streamable_http_app()` returns a `Router` that responds to `GET /health` with 200.
   - `build_streamable_http_app()` returns a `Router` that responds to `GET /metrics` (404 without exporter, 200 with).
   - `build_streamable_http_app()` with extra routes includes those routes.
   - `run_streamable_http()` rejects empty host.
   - `run_streamable_http()` rejects port 0 (if we validate that).
2. **Implement `build_streamable_http_app()`:**
   ```rust
   pub fn build_streamable_http_app(
       self: &Arc<Self>,
       handler: Arc<dyn McpHandler>,
       extra_routes: Option<Router>,
   ) -> Router {
       let mcp_router = /* MCP endpoint router handling POST/GET/DELETE at / */;
       let mut app = self.health_metrics_router()
           .nest("/mcp", mcp_router);
       if let Some(extra) = extra_routes {
           app = app.merge(extra);
       }
       app
   }
   ```
3. **Implement the MCP endpoint handler** at `/mcp`:
   - POST: accept JSON-RPC request, delegate to `McpHandler`, return JSON-RPC response.
   - GET: streamable HTTP (SSE-based streaming for server-to-client notifications). Use `axum::response::Sse`.
   - DELETE: session termination (return 200).
   - Store a session ID (`uuid::Uuid::new_v4().to_string()`) in state.
4. **Implement `run_streamable_http()`:**
   ```rust
   pub async fn run_streamable_http(
       self: &Arc<Self>,
       handler: Arc<dyn McpHandler>,
       host: &str,
       port: u16,
       extra_routes: Option<Router>,
   ) -> Result<(), TransportError> {
       Self::validate_host_port(host, port)?;
       tracing::info!("Starting streamable-http transport on {}:{}", host, port);

       let app = self.build_streamable_http_app(handler, extra_routes);
       let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse()
           .map_err(|_| TransportError::InvalidHost(host.to_string()))?;
       let listener = tokio::net::TcpListener::bind(addr).await
           .map_err(|e| TransportError::Io(e))?;
       axum::serve(listener, app).await
           .map_err(|e| TransportError::Io(e))?;
       Ok(())
   }
   ```
5. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] `build_streamable_http_app()` returns a `Router` with /health, /metrics, /mcp
- [ ] `/mcp` handles POST with JSON-RPC request/response
- [ ] Session ID is generated per transport instance via `uuid::Uuid::new_v4()`
- [ ] Extra routes are merged into the router
- [ ] `run_streamable_http()` validates host and port
- [ ] `run_streamable_http()` binds to the specified address and serves
- [ ] Logs transport start at info level
- [ ] Tests pass (using `tower::ServiceExt::oneshot` for router testing)

## Dependencies

- implement-health-metrics

## Estimated Time

2 hours
