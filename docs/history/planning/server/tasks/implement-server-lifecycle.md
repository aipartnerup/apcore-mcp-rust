# Task: implement-server-lifecycle

## Goal

Implement the `start()`, `wait()`, and `stop()` methods on `MCPServer` using `tokio::spawn`, oneshot signaling for "started" notification, and watch channel for graceful shutdown.

## Files Involved

- `src/server/server.rs` — implement `start()`, `wait()`, `stop()`, and internal `_run()` method

## Steps (TDD-first)

1. **Write tests first:**
   - `start()` on an already-started server is a no-op (idempotent).
   - `stop()` on a not-started server is a no-op.
   - `start()` followed by `stop()` does not panic.
   - After `stop()`, `wait()` completes.
   - (Mock/stub the transport layer so tests do not bind real ports.)
2. **Create shutdown channel in `start()`:**
   ```rust
   let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
   self.shutdown_tx = Some(shutdown_tx);
   ```
3. **Create started oneshot in `start()`:**
   ```rust
   let (started_tx, started_rx) = tokio::sync::oneshot::channel::<()>();
   ```
4. **Implement the spawned task (`_run` logic):**
   - Resolve registry and executor from `self.registry_or_executor`.
   - Create factory, server, tools, router via `MCPServerFactory`.
   - Register handlers and resource handlers.
   - Build init options.
   - Conditionally build auth middleware (if authenticator provided and HTTP transport).
   - Create `TransportManager`.
   - Send `started_tx.send(())` to signal ready.
   - Match on `TransportKind` and call the appropriate `transport_manager.run_*()` method.
   - Use `tokio::select!` to race the transport future against `shutdown_rx.changed()`.
   - On shutdown signal, exit cleanly.
5. **Store the `JoinHandle`:**
   ```rust
   self.join_handle = Some(tokio::spawn(async move { ... }));
   ```
6. **Await started signal with timeout:**
   ```rust
   tokio::time::timeout(Duration::from_secs(10), started_rx).await??;
   ```
7. **Implement `wait()`:**
   ```rust
   if let Some(handle) = self.join_handle.take() {
       handle.await??;
   }
   ```
8. **Implement `stop()`:**
   ```rust
   if let Some(tx) = &self.shutdown_tx {
       let _ = tx.send(true);
   }
   ```
9. **Make `start()` idempotent:** Check if `join_handle.is_some()` before spawning.
10. **Remove all `todo!()` macros** from `start()`, `wait()`, `stop()`.
11. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] `start()` spawns a background task via `tokio::spawn`
- [ ] `start()` awaits a oneshot "started" signal with 10-second timeout
- [ ] `start()` is idempotent (second call is a no-op)
- [ ] `wait()` awaits the `JoinHandle` until task completes
- [ ] `stop()` sends shutdown signal via watch channel
- [ ] `stop()` on unstarted server is a no-op
- [ ] Spawned task uses `tokio::select!` to race transport vs shutdown
- [ ] Auth middleware is conditionally built for HTTP transports
- [ ] All `todo!()` macros removed from lifecycle methods
- [ ] All tests pass

## Dependencies

- implement-mcp-server-struct

## Estimated Time

2 hours
