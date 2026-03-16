# Task: implement-mcp-server-struct

## Goal

Implement the `MCPServer` struct with its constructor, accepting `RegistryOrExecutor` and `MCPServerConfig`, and storing all necessary state for the lifecycle methods. Define the `RegistryOrExecutor` enum.

## Files Involved

- `src/server/server.rs` — update `MCPServer` struct fields, implement `new()`, define `RegistryOrExecutor`

## Steps (TDD-first)

1. **Write tests first:**
   - `MCPServer::new()` with Stdio transport returns `address() == "stdio"`.
   - `MCPServer::new()` with StreamableHttp transport returns `address() == "http://127.0.0.1:8000"`.
   - `MCPServer::new()` with custom host/port returns correct address.
   - Server is not started after construction (internal state reflects "not running").
2. **Define `RegistryOrExecutor` enum:**
   ```rust
   pub enum RegistryOrExecutor {
       Registry(/* apcore::Registry or trait object */),
       Executor(/* apcore::Executor or trait object */),
   }
   ```
   Note: The exact types depend on the `apcore` crate's public API. Use trait objects (`Arc<dyn ...>`) if needed.
3. **Update `MCPServer` struct:**
   ```rust
   pub struct MCPServer {
       config: MCPServerConfig,
       registry_or_executor: RegistryOrExecutor,
       // Lifecycle state (populated by start()):
       join_handle: Option<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send>>>>,
       shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
   }
   ```
4. **Implement `MCPServer::new(registry_or_executor, config)`:**
   - Store config and input.
   - Initialize `join_handle` and `shutdown_tx` as `None`.
5. **Implement `MCPServer::address()`:**
   - Delegate to `self.config.transport.address(&self.config.host, self.config.port)`.
6. **Remove old constructor** that takes individual `name`, `transport`, `host`, `port` parameters.
7. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] `RegistryOrExecutor` enum is defined with Registry and Executor variants
- [ ] `MCPServer::new()` accepts `RegistryOrExecutor` and `MCPServerConfig`
- [ ] `address()` delegates to `TransportKind::address()`
- [ ] Lifecycle state fields (`join_handle`, `shutdown_tx`) are initialized as `None`
- [ ] Old constructor is removed
- [ ] All tests pass

## Dependencies

- define-transport-kind-enum
- define-server-config
- implement-registry-listener

## Estimated Time

1 hour
