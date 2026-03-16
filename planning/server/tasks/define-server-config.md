# Task: define-server-config

## Goal

Define `MCPServerConfig` to collect the many optional parameters for MCPServer construction (host, port, name, version, validate_inputs, metrics_collector, tags, prefix, authenticator, require_auth, exempt_paths) into a single config struct with sensible defaults.

## Files Involved

- `src/server/server.rs` — add `MCPServerConfig` struct, update `MCPServer::new()`

## Steps (TDD-first)

1. **Write tests first:**
   - `MCPServerConfig::default()` has expected defaults: host="127.0.0.1", port=8000, name="apcore-mcp", transport=Stdio, validate_inputs=false, require_auth=true.
   - Config can be constructed with builder-style methods.
   - `MCPServer::new()` accepts config and stores fields correctly.
2. **Define the config struct:**
   ```rust
   pub struct MCPServerConfig {
       pub transport: TransportKind,
       pub host: String,
       pub port: u16,
       pub name: String,
       pub version: Option<String>,
       pub validate_inputs: bool,
       pub tags: Option<Vec<String>>,
       pub prefix: Option<String>,
       pub require_auth: bool,
       pub exempt_paths: Option<HashSet<String>>,
       // authenticator and metrics_collector use trait objects:
       // pub authenticator: Option<Arc<dyn Authenticator>>,
       // pub metrics_collector: Option<Arc<dyn MetricsExporter>>,
   }
   ```
3. **Implement `Default`** with values matching the Python defaults:
   - `transport`: `TransportKind::Stdio`
   - `host`: `"127.0.0.1"`
   - `port`: `8000`
   - `name`: `"apcore-mcp"`
   - `version`: `None`
   - `validate_inputs`: `false`
   - `require_auth`: `true`
   - Others: `None`
4. **Update `MCPServer::new()`** to accept `RegistryOrExecutor` and `MCPServerConfig`.
5. **Derive `Debug, Clone`** on `MCPServerConfig` (where possible; trait object fields may need manual Debug).
6. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] `MCPServerConfig` collects all optional parameters
- [ ] `Default` implementation matches Python defaults
- [ ] `MCPServer::new()` takes config instead of individual parameters
- [ ] Config fields are accessible for the server lifecycle implementation
- [ ] All tests pass

## Dependencies

- define-transport-kind-enum

## Estimated Time

45 minutes
