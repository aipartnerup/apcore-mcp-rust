# Feature: MCP Server

## Module Purpose
High-level server orchestrator that combines factory, router, transport, and listener into a managed server lifecycle with background thread execution.

## Public API Surface

### MCPServer
- `new(registry_or_executor, transport, host, port, name, version, validate_inputs, metrics_collector, tags, prefix, authenticator, require_auth, exempt_paths) -> MCPServer`
- `start()` — non-blocking, spawns background task
- `wait()` — blocking, waits for server to finish
- `stop()` — graceful shutdown
- `address() -> String` — "stdio" or "http://{host}:{port}"

### RegistryListener
- `new(registry, factory) -> RegistryListener`
- `start()` — idempotent, begins listening for registry events
- `stop()`
- `tools() -> HashMap<String, Tool>` — thread-safe snapshot

## Acceptance Criteria
- [ ] Accepts either Registry or Executor as input
- [ ] Resolves registry/executor from input (resolve_registry/resolve_executor)
- [ ] Creates server, router, factory, transport, and listener
- [ ] start() spawns server in background (non-blocking)
- [ ] wait() blocks until server terminates
- [ ] stop() triggers graceful shutdown
- [ ] address() returns correct address string
- [ ] RegistryListener reacts to register/unregister events
- [ ] RegistryListener provides thread-safe tool snapshot
- [ ] Applies authentication middleware when authenticator is provided
