# Task: refactor-transport-manager-struct

## Goal

Refactor the `TransportManager` struct to match the Python implementation: accept an optional external `MetricsExporter`, track start time, and remove the incorrect `MetricsExporter` self-implementation.

## Files Involved

- `src/server/transport.rs` — modify struct and constructor

## Steps (TDD-first)

1. **Write tests first:**
   - `TransportManager::new(None)` creates a manager with no metrics exporter.
   - `TransportManager::new(Some(exporter))` stores the exporter.
   - `set_module_count(5)` updates the count retrievable by health response.
   - `module_count` defaults to 0.
2. **Refactor `TransportManager` struct:**
   ```rust
   pub struct TransportManager {
       start_time: tokio::time::Instant,
       module_count: usize,
       metrics_exporter: Option<Arc<dyn MetricsExporter + Send + Sync>>,
   }
   ```
3. **Update constructor:**
   ```rust
   impl TransportManager {
       pub fn new(metrics_exporter: Option<Arc<dyn MetricsExporter + Send + Sync>>) -> Self {
           Self {
               start_time: tokio::time::Instant::now(),
               module_count: 0,
               metrics_exporter,
           }
       }
   }
   ```
4. **Keep `set_module_count()` as-is.**
5. **Remove `impl MetricsExporter for TransportManager` block.** The `TransportManager` is not itself a metrics exporter; it delegates to an injected one.
6. **Update method signatures** on `run_stdio`, `run_streamable_http`, `run_sse`, `build_streamable_http_app` to return `Result<(), TransportError>` instead of `Result<(), Box<dyn std::error::Error>>`.
7. **Add `validate_host_port()` private method:**
   ```rust
   fn validate_host_port(host: &str, port: u16) -> Result<(), TransportError> {
       if host.is_empty() {
           return Err(TransportError::InvalidHost(host.to_string()));
       }
       // port is u16 so 0 is the only invalid value (0 means OS-assigned, which is valid)
       // Match Python: 1..=65535 — but u16 max is 65535, so only reject 0
       if port == 0 {
           return Err(TransportError::InvalidPort(port));
       }
       Ok(())
   }
   ```
8. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `TransportManager::new()` accepts `Option<Arc<dyn MetricsExporter + Send + Sync>>`
- [ ] `start_time` is set to `Instant::now()` at construction
- [ ] `module_count` defaults to 0
- [ ] `impl MetricsExporter for TransportManager` is removed
- [ ] All public methods return `Result<(), TransportError>`
- [ ] `validate_host_port()` rejects empty host
- [ ] Tests pass

## Dependencies

- none

## Estimated Time

30 minutes
