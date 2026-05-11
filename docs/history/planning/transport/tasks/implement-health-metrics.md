# Task: implement-health-metrics

## Goal

Implement the health check and Prometheus metrics HTTP endpoint handlers as axum handler functions. These are shared across all HTTP transports (streamable-http and SSE).

## Files Involved

- `src/server/transport.rs` — add handler functions and helper methods

## Steps (TDD-first)

1. **Write tests first:**
   - `build_health_response()` returns JSON with `status: "ok"`, `uptime_seconds` (>= 0.0), and `module_count`.
   - `build_health_response()` after `set_module_count(3)` returns `module_count: 3`.
   - `build_metrics_response()` with no exporter returns 404 status.
   - `build_metrics_response()` with a mock exporter returns 200 with `text/plain` content type and the exporter's output.
2. **Define health response struct:**
   ```rust
   #[derive(serde::Serialize)]
   struct HealthResponse {
       status: &'static str,
       uptime_seconds: f64,
       module_count: usize,
   }
   ```
3. **Implement `TransportManager::build_health_response()`:**
   ```rust
   fn build_health_response(&self) -> HealthResponse {
       HealthResponse {
           status: "ok",
           uptime_seconds: self.start_time.elapsed().as_secs_f64(),
           module_count: self.module_count,
       }
   }
   ```
4. **Implement axum health handler** (as a function that takes `State<Arc<TransportManager>>`):**
   ```rust
   async fn health_handler(
       State(tm): State<Arc<TransportManager>>,
   ) -> axum::Json<HealthResponse> {
       axum::Json(tm.build_health_response())
   }
   ```
5. **Implement axum metrics handler:**
   ```rust
   async fn metrics_handler(
       State(tm): State<Arc<TransportManager>>,
   ) -> axum::response::Response {
       match &tm.metrics_exporter {
           Some(exporter) => {
               let body = exporter.export_prometheus();
               (
                   StatusCode::OK,
                   [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
                   body,
               ).into_response()
           }
           None => StatusCode::NOT_FOUND.into_response(),
       }
   }
   ```
6. **Implement `TransportManager::health_metrics_router()`** helper that returns a `Router` with `/health` and `/metrics` GET routes, using `Arc<TransportManager>` as state.
7. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] Health response contains `status`, `uptime_seconds`, `module_count`
- [ ] `uptime_seconds` is computed from `start_time.elapsed()`
- [ ] Metrics endpoint returns 200 with Prometheus text when exporter is present
- [ ] Metrics endpoint returns 404 when no exporter is configured
- [ ] Content-Type is `text/plain; version=0.0.4; charset=utf-8` for metrics
- [ ] `health_metrics_router()` returns a Router with GET /health and GET /metrics
- [ ] All tests pass

## Dependencies

- define-transport-error
- refactor-transport-manager-struct

## Estimated Time

45 minutes
