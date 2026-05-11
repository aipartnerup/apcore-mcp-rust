# Task: implement-sse-transport

## Goal

Implement `run_sse()` to serve MCP over SSE transport (deprecated). Mounts endpoints at `/sse` (GET, server-to-client event stream) and `/messages/` (POST, client-to-server messages). Health and metrics endpoints are auto-registered.

## Files Involved

- `src/server/transport.rs` — implement `run_sse`

## Steps (TDD-first)

1. **Write tests first:**
   - SSE router responds to `GET /health` with 200.
   - SSE router responds to `GET /metrics` (404 without exporter).
   - `run_sse()` rejects empty host.
   - `run_sse()` logs a deprecation warning.
2. **Implement SSE shared state:**
   - Create a `tokio::sync::mpsc` channel pair for client-to-server messages.
   - The POST `/messages/` handler sends into the channel.
   - The GET `/sse` handler reads from the channel, processes via `McpHandler`, and streams responses as SSE events.
3. **Implement GET `/sse` handler:**
   ```rust
   async fn sse_handler(
       State(state): State<SseState>,
   ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
       // Create a stream that reads from the mpsc receiver,
       // processes each message through the MCP handler,
       // and yields SSE Event items.
   }
   ```
4. **Implement POST `/messages/` handler:**
   ```rust
   async fn messages_handler(
       State(state): State<SseState>,
       body: axum::Json<serde_json::Value>,
   ) -> StatusCode {
       // Send the message into the mpsc sender.
       // Return 202 Accepted.
   }
   ```
5. **Implement `run_sse()`:**
   ```rust
   pub async fn run_sse(
       self: &Arc<Self>,
       handler: Arc<dyn McpHandler>,
       host: &str,
       port: u16,
       extra_routes: Option<Router>,
   ) -> Result<(), TransportError> {
       Self::validate_host_port(host, port)?;
       tracing::info!("Starting sse transport on {}:{}", host, port);
       tracing::warn!("SSE transport is deprecated. Use Streamable HTTP instead.");

       let (tx, rx) = tokio::sync::mpsc::channel(256);
       let sse_state = SseState { handler, sender: tx, receiver: Arc::new(Mutex::new(rx)) };

       let mut app = self.health_metrics_router()
           .route("/sse", get(sse_handler))
           .route("/messages/", post(messages_handler))
           .with_state(sse_state);
       if let Some(extra) = extra_routes {
           app = app.merge(extra);
       }

       let addr: std::net::SocketAddr = format!("{}:{}", host, port).parse()
           .map_err(|_| TransportError::InvalidHost(host.to_string()))?;
       let listener = tokio::net::TcpListener::bind(addr).await?;
       axum::serve(listener, app).await.map_err(|e| TransportError::Io(e))?;
       Ok(())
   }
   ```
6. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] GET `/sse` returns an SSE event stream
- [ ] POST `/messages/` accepts JSON-RPC and returns 202
- [ ] Client-to-server messages are relayed to the MCP handler
- [ ] MCP handler responses are streamed back as SSE events
- [ ] Health and metrics endpoints are registered
- [ ] Deprecation warning is logged at warn level
- [ ] Extra routes are merged
- [ ] Host/port validation is performed
- [ ] Tests pass

## Dependencies

- implement-health-metrics

## Estimated Time

1.5 hours
