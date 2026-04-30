# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.14.0] - 2026-04-28

Leverages apcore 0.19.0 + apcore-toolkit 0.5.0. Wires three apcore modules
that the bridge previously did not use: `trace_context`, `async_task`, and
`observability::{metrics,usage}`. Aligns the Rust bridge with the Python
and TypeScript 0.14.0 implementations.

### Changed

- **`apcore` dependency bumped from `"0.17"` to `"0.19"`** (actual resolution jumps
  from 0.18.0 to 0.19.0).
- **New dependency: `apcore-toolkit = "0.5"`** — brings `BindingLoader` /
  `BindingLoadError` (pure-data reader for `.binding.yaml` with safety caps:
  16 MiB per file, 10 000 files per dir) and `ScannedModule.display` into the
  MCP bridge's dependency surface for downstream callers that need to hydrate
  modules from declarative bindings.
- **`ModuleDescriptor` struct literals** in `src/apcore_mcp.rs`,
  `src/server/factory.rs`, `src/server/listener.rs`, and `examples/run/main.rs`
  updated to supply the new `display: None` field (apcore 0.19.0 breaking
  change).
- **`APCoreMCPBuilder::build()` ACL install path** — apcore 0.19.0's
  `Executor::set_acl` takes `&mut self`. The builder now calls `Arc::get_mut`
  on the resolved executor; if the `Arc` is already shared (caller passed a
  clone), the builder returns a `Config` error pointing to the remediation
  (install ACL on the `Executor` before wrapping it in `Arc`). Affects the
  `BackendSource::Executor(Arc<Executor>) + .acl(...)` flow.
- **`ACL::check()` call site** in `tests/acl_conformance.rs` — returns `bool`
  directly in 0.19.0 (was `Result<bool, _>`).
- **Executor `acl()` accessor** — now a public field (`exec.acl`) rather than a
  method in apcore 0.19.0; updated in the two test assertions that inspected it.
- **`ExecutionRouter` state** carries `async_bridge: Option<Arc<_>>` and
  `async_module_ids: HashSet<String>`. Non-async paths are bit-for-bit
  unchanged when the bridge is not attached.

### Added

- **W3C Trace Context propagation** (P0). `src/server/router.rs` now imports
  `apcore::trace_context::{TraceContext, TraceParent}`, parses inbound
  `_meta.traceparent` on `tools/call` requests, and threads the resulting
  trace id through the `apcore::Context` so downstream module invocations
  inherit the W3C trace chain. Outbound tool responses carry
  `_meta.traceparent` built via `TraceContext::inject(context)`, so MCP
  clients can correlate spans across the bridge without bespoke plumbing.
  Malformed headers are rejected by apcore's strict validator (the bridge
  does not duplicate that logic).
- **Async Task Bridge** (`src/server/async_task_bridge.rs`, new, F-043 per
  `docs/features/async-task-bridge.md`). Exposes apcore's
  `AsyncTaskManager` through MCP so long-running modules can be submitted,
  polled, cancelled, and listed without blocking the transport.
  - `AsyncTaskBridge` struct with `is_async_module` (checks
    `metadata.async == true` OR `annotations.extra.mcp_async == "true"`),
    `submit`, `get_status`, `cancel`, `cancel_session_tasks`, `list_tasks`,
    `shutdown`, plus `is_reserved_id` and `is_async_registered` helpers.
  - Four reserved meta-tools registered under the `__apcore_task_` prefix:
    `__apcore_task_submit`, `__apcore_task_status`, `__apcore_task_cancel`,
    `__apcore_task_list`. `MCPServerFactory::build_tools` rejects any
    user-registered module id that collides with the reserved prefix.
  - `ExecutionRouter::with_async_bridge(bridge, async_ids)` installs the
    bridge; the router routes async-hinted module ids through
    `AsyncTaskManager::submit` instead of the synchronous executor path
    and returns a `{task_id, status: "pending"}` envelope immediately.
  - Progress fan-out: when the caller supplies `_meta.progressToken`,
    module-side `report_progress(context, ...)` calls flow through as
    MCP `notifications/progress` tied to the submitting session.
  - Status projection redacts sensitive fields via `redact_sensitive` using
    the router's `output_schemas` map so completed results respect the
    same schema-driven masking as the sync path.
  - `TaskLimitExceededError` (apcore 0.19.0) is routed through the
    existing error mapper with `retryable: true`.
- **TransportManager cancellation forwarding**
  (`src/server/transport.rs`). New `set_cancel_handler` /
  `notify_cancel(session_id)` hook. `APCoreMCPBuilder::async_serve` /
  `serve` wire the handler to `AsyncTaskBridge::cancel_session_tasks` so
  client disconnects cancel any tasks submitted from that session.
- **Observability auto-wiring** (P0). New `observability: bool` field on
  `APCoreMCPConfig` + `--observability` CLI flag + `.observability(true)`
  builder method.
  - When enabled, `APCoreMCPBuilder::build` auto-instantiates
    `apcore::observability::metrics::MetricsCollector` and
    `apcore::observability::usage::UsageCollector` and installs
    `MetricsMiddleware` + `UsageMiddleware` on the executor. The
    transport's `/metrics` endpoint (already exposed via the existing
    `MetricsExporter` Protocol) now has a real source out of the box.
  - Blanket `impl MetricsExporter for apcore::…::MetricsCollector` so the
    apcore collector plugs directly into the bridge's existing metrics
    surface without an adapter type.
  - New `UsageExporter` trait + blanket impl for apcore's `UsageCollector`.
    Adds `/usage` endpoint to `TransportManager` returning per-module
    summaries (call count, error count, latency, unique callers, trend)
    as JSON. Endpoint returns 404 when no usage exporter is configured.
  - A pre-instantiated custom `MetricsExporter` passed by the caller is
    preserved untouched — auto-wiring only kicks in for the
    `observability=true` / `metrics=true` zero-config path.
- **Type-safe error dispatch** — `src/adapters/errors.rs` now matches the
  new apcore 0.19.0 `ModuleError` variants (`TaskLimitExceeded`,
  `DependencyNotFound`, `DependencyVersionMismatch`,
  `BindingSchemaInferenceFailed`, `BindingSchemaModeConflict`,
  `BindingStrictSchemaIncompatible`, `BindingPolicyViolation`,
  `VersionConstraintInvalid`) with explicit arms instead of relying only
  on error-code string matches, tightening cross-language contracts.
- **8 new `ErrorCode` variants** surfacing apcore 0.19.0 protocol additions:
  `DependencyNotFound`, `DependencyVersionMismatch`, `TaskLimitExceeded`,
  `VersionConstraintInvalid`, `BindingSchemaInferenceFailed`,
  `BindingSchemaModeConflict`, `BindingStrictSchemaIncompatible`,
  `BindingPolicyViolation`. Total variants: 35 (was 27).
- **Dependency-error mapping in `ErrorMapper`** — `DependencyNotFound` and
  `DependencyVersionMismatch` now render a structured, agent-friendly message
  extracted from `details.module_id` / `dependency_id` / `required` / `actual`
  so MCP clients don't have to parse the detail bag.
- **Binding-configuration error routing** — `BindingSchema*` / `TaskLimitExceeded`
  / `VersionConstraintInvalid` are explicitly routed through `build_detail_response`
  (detail passthrough + AI guidance attachment) rather than hitting the default
  branch.
- **Expanded annotation surface in `AnnotationMapper::to_description_suffix`** —
  `cache_ttl`, `cache_key_fields`, `pagination_style` are now rendered into the
  `[Annotations: ...]` block when set to non-default values. `annotations.extra`
  keys prefixed with `mcp_` are passed through verbatim (F-041, previously
  blocked on apcore exposing `extra`).
- **Top-level `ModuleDescriptor.display` precedence** in `MCPServerFactory::build_tools`.
  The 0.19.0 descriptor adds a canonical `display: Option<Value>` field; it now
  takes precedence over the legacy `metadata["display"]` overlay (still honored
  for backwards compatibility).

### Tests

- **788 tests pass** (`cargo test --all-features`): 771 lib + 2 acl + 1
  adapters + 1 auth + 6 cli + 1 converters + 2 middleware + 1 server + 3
  doc. Up from 756 before this release.
- New unit coverage added inline under `#[cfg(test)]` in
  `src/server/async_task_bridge.rs` (hint detection, reserved-id
  rejection, submit/status/cancel/list, meta-tool schema, session
  cancellation), `src/server/router.rs` (traceparent parse + trace-id
  propagation + outbound `_meta.traceparent`), `src/server/transport.rs`
  (usage endpoint JSON shape, 404 without exporter, cancel handler
  invocation), and `src/apcore_mcp.rs` (observability flag auto-wires
  collectors; disabled path wires nothing; blanket `MetricsExporter` impl
  routes to `MetricsCollector::export_prometheus`).
- `error_code_count` guard updated: 27 → 35.
- `all_python_error_codes_parse` fixture extended with the 8 new canonical names.

### Cross-language sync (deferred-modules round, 2026-04-28)

- **Dependency bump**: `mcp-embedded-ui = "0.4"` (was `"0.3"`). The new release ships `POST /tools/{name}/validate` (F7) — read-only schema validation, ungated by `allow_execute`, `auth_hook`, or `Authenticator`. The route flows automatically through the existing `mcp_embedded_ui::create_mount` adapter in `src/explorer/mount.rs`. **Resolves EUI-1.** TC-011 integration tests added in `src/explorer/mount.rs::tests`.
- **OC-5 (BREAKING) — `OpenAIConverter::convert_registry` signature.** The canonical entrypoint now takes `&apcore::registry::Registry` directly (matching Python+TS duck-typed Registry input). The pre-fix `&serde_json::Value` snapshot variant is preserved as `convert_registry_json` for callers that hold a serialized snapshot:
  ```rust
  // Live registry path (preferred):
  converter.convert_registry(&registry, false, false, None, None)?;

  // Or keep using a JSON snapshot:
  converter.convert_registry_json(&value, false, false, None, None)?;
  ```
  `APCoreMCP::to_openai_tools` switched to the live-registry path, dropping the unused `build_registry_json` helper. 4 regression tests added.
- **AH-1 — per-request elicit callback via task-local.** Added `tokio::task_local! ELICIT_CALLBACK` in `apcore_mcp::helpers`. `ElicitationApprovalHandler::request_approval` now resolves the callback from the task-local first (matching Python+TS, which read it from `context.data`), with the constructor field as a fallback. apcore-rust's `Context::data` (`HashMap<String, serde_json::Value>`) cannot hold boxed `Fn`s, so a task-local is the closest cross-SDK equivalent without forcing an apcore-rust extension. 4 regression tests.
- **EM-3 — `userFixable=true` stamp** for `DependencyNotFound`, `DependencyVersionMismatch`, `VersionConstraintInvalid`, and the four `Binding*` codes (matches TS). Added `USER_FIXABLE_ERROR_CODES` const + stamp in `build_detail_response`. 5 regression tests.
- **EM-6 — generic-error fallback.** `ErrorMapper::internal_error_response()` and `ErrorMapper::to_mcp_error_any<E: std::error::Error>()` return the canonical `{is_error:true, error_type:"GENERAL_INTERNAL_ERROR", message:"Internal error occurred", details:null}` envelope for any non-`ModuleError` input — matches Python's `to_mcp_error(error: Exception)` and TypeScript's `toMcpError(error: unknown)`. 3 regression tests.
- **MID-5 — `ModuleIDNormalizer::denormalize_checked`.** Bijection-guarded variant validates the dash→dot-replaced result against the canonical module-id pattern, returning `Err(InvalidModuleId)` for inputs that aren't valid pre-images of `normalize`. Plain `denormalize` stays lenient. 5 regression tests.
- **SC-9 / SC-18** — strict-schema walker now stops descending into `enum` / `const` / `examples` / `default` and preserves `type: ["object", "null"]` (no longer downgrades to bare `"object"`). Output now matches Python+TS.
- **AM-L1 — F-041 annotation extras format aligned with Python+TS.** `mcp_*` extras are now emitted as separate `<stripped-key>: <value>` lines appended after the `[Annotations: ...]` block, separated by a single newline. Pre-fix Rust inlined them into the `[Annotations: ...]` block as `mcp_key=value`, which diverged from the other two SDKs on the wire. 1 regression test.

#### Deferred to a future release

- **A-D-012** — canonical strict-schema sourcing via `apcore::Registry::export_schema_strict` (committed locally as `62706be` but not yet on crates.io). 0.14.0 ships with the local-`SchemaConverter` fallback as the canonical path; behaviour is identical, the upgrade is purely about delegating to apcore upstream when the new release lands.
- **EB-2 (Rust)** — adapter-hook injection (`schema_converter` / `annotation_mapper` / `error_mapper` overrides on `serve()`). Blocked on `SchemaConverter` and `AnnotationMapper` being stateless unit structs with only static methods; needs a trait-based redesign first. Python+TS already ship the kwargs.

---

## [0.13.0] - 2026-04-06

### Added

- **Pipeline Strategy Selection** (F-036) — `--strategy` CLI flag and builder `.strategy()` with 5 presets (standard, internal, testing, performance, minimal).
- **Tool Output Redaction** (F-038) — `redact_output` config (default: true) applies `redact_sensitive()` before serialization.
- **Pipeline Observability** (F-037) — `.trace(true)` enables `call_with_trace()` for per-step timing.
- **Tool Preflight Validation** (F-039) — `ExecutionRouter::validate_tool()` for dry-run validation.
- **YAML Pipeline Configuration** (F-040) — Config Bus `mcp.pipeline` section via `build_strategy_from_config()`.
- **Annotation Metadata Passthrough** (F-041) — `mcp_` prefixed keys from annotations extra (behind feature flag).
- **4 new error mappings** — `CONFIG_ENV_MAP_CONFLICT`, `PIPELINE_ABORT`, `STEP_NOT_FOUND`, `VERSION_INCOMPATIBLE`.
- **RegistryListener wired to `dynamic` serve option**.

### Changed

- **Dependency bump**: `apcore = "0.17"` (was `"0.15"`).

---

## [0.12.0] - 2026-03-31

### Added

- **Config Bus namespace registration** (F-033) — Registers `mcp` namespace with apcore Config Bus (`APCORE_MCP` env prefix) during `APCoreMCPBuilder::build()`. MCP configuration (transport, host, port, auth, explorer) can be managed via unified `apcore.yaml`.
- **Error Formatter Registry integration** (F-034) — `McpErrorFormatter` registered with apcore's `ErrorFormatterRegistry`, formalizing MCP error formatting into the shared protocol.
- **Dot-namespaced event constants** (F-035) — `apcore_events` module with canonical event type constants from apcore 0.15.0 (§9.16).
- **6 new error code variants** — `ConfigNamespaceDuplicate`, `ConfigNamespaceReserved`, `ConfigEnvPrefixConflict`, `ConfigMountError`, `ConfigBindError`, `ErrorFormatterDuplicate`.

### Changed

- Dependency bump: requires `apcore 0.15.1` (was `0.14`) for Config Bus (§9.4), Error Formatter Registry (§8.8), and dot-namespaced event types (§9.16).

---

## [0.11.1] - 2026-03-29

### Added
- **Context.data callback injection** — `build_context()` now constructs a proper `apcore::Context<Value>` and injects MCP callback markers (`_mcp_progress`, `_mcp_elicit`) into `Context.data` (SharedData). Actual callbacks stored in a side-channel `HashMap<String, Box<dyn Any>>` since `serde_json::Value` cannot hold function pointers. Modules can detect callback availability via marker values.
- **Identity propagation** — `build_context()` resolves identity with a priority chain: `CallExtra.typed_identity` > deserialized JSON identity > `AUTH_IDENTITY` task-local from auth middleware. Resolved identity is used with `Context::new(identity)` or `Context::anonymous()`.
- **`redact_sensitive()` logging** — Added `tool_schemas` field and `with_tool_schemas()` builder method to `ExecutionRouter`. Tool inputs are redacted via `apcore::redact_sensitive()` before debug logging, replacing `x-sensitive: true` fields and `_secret_*` prefixed keys with `***REDACTED***`.
- **`CallExtra.typed_identity`** field for direct typed identity injection (bypasses JSON deserialization).
- 12 new tests: `build_context` identity resolution (4), callback marker injection (4), redact_sensitive (3), builder (1).

### Changed
- `build_context()` now returns a 3-tuple `(context_value, callback_data, apcore_context)` instead of a 2-tuple, providing the constructed `apcore::Context` for downstream use.
- JSON context `trace_id` is now taken from the `apcore::Context` for consistency.

- Bump apcore dependency from 0.13 to 0.14. All 694 tests pass without code changes — apcore 0.14 breaking changes (Context.identity optional, SharedData, middleware priority default 100) are backward-compatible for apcore-mcp.

## [0.11.0] - 2026-03-26

### Added
- **Display overlay in `build_tool()`** — MCP tool name, description, and guidance now sourced from `metadata["display"]["mcp"]` when present.
  - Tool name: `metadata["display"]["mcp"]["alias"]` (pre-sanitized by `DisplayResolver`, already `[a-zA-Z_][a-zA-Z0-9_-]*` and ≤ 64 chars).
  - Tool description: `metadata["display"]["mcp"]["description"]`, with `guidance` appended as `\n\nGuidance: <text>` when set.
  - Falls back to raw `descriptor.module_id` / `descriptor.description` when no display overlay is present.
- `build_tool()` now accepts `name_override` parameter for display overlay tool names.
- `build_tools_with_metadata()` method for resolving display overlay from module metadata.

### Changed
- Dependency recommendation: works best with `apcore-toolkit >= 0.4.0` for `DisplayResolver`.

### Fixed
- README: corrected `mcp.serve(Default::default())` to `mcp.serve()` (zero-argument method).
- README: updated apcore version requirement from `>= 0.13.0` to `>= 0.14.0`.
- docs/features: updated function signatures to use config structs (`ServeConfig`, `AsyncServeConfig`, `OpenAIToolsConfig`).

### Tests
- `TestBuildToolDisplayOverlay` (8 tests): MCP alias used as tool name, MCP description used, guidance appended to description, fallback when no overlay, fallback with empty metadata, name_override direct test, all fields combined.

## [0.10.1] - 2026-03-22

### Changed
- Rebrand: aipartnerup → aiperceivable

## [0.10.0] - 2026-03-17

### Added
- Initial project scaffolding: core modules, CLI, server, authentication, and comprehensive planning.
- **MCP server** with stdio, Streamable HTTP, and SSE transport support.
- **MCPServerFactory** for building tools, resources, and initialization options from an apcore registry.
- **ExecutionRouter** for dispatching tool calls with streaming, progress reporting, elicitation, and output formatting.
- **TransportManager** with health/metrics endpoints and Prometheus observability.
- **RegistryListener** for dynamic tool registration/unregistration via registry events.
- **JWTAuthenticator** with configurable claim mapping, algorithm selection, and key file support.
- **AuthMiddlewareLayer** (Tower layer) for HTTP request authentication with `AUTH_IDENTITY` task-local propagation.
- **Adapters**: AnnotationMapper, SchemaConverter (with `$ref`/`$defs` inlining), ErrorMapper, ModuleIDNormalizer, ElicitationApprovalHandler.
- **OpenAIConverter** for translating apcore registries to OpenAI function-calling format with strict mode support.
- **Explorer UI** powered by `mcp-embedded-ui` crate, with AuthBridge for identity propagation between apcore and the UI layer.
- **CLI** (`apcore-mcp`) with `--transport`, `--host`, `--port`, `--extensions-dir`, `--tags`, `--prefix`, and `--jwt-*` flags.
- **Helper functions**: `report_progress` and `elicit` for MCP progress notifications and user elicitation.
- **Constants**: `ErrorCode` and `RegistryEvent` enums with strum-based serialization matching the Python SDK wire format.
- **APCoreMCPBuilder** for fluent construction with backend, authenticator, metrics, output formatter, and approval handler.
- Convenience functions: `serve()`, `async_serve()`, `to_openai_tools()`.
- `Makefile` with `setup`, `check`, `test`, `lint`, `fmt`, and `clean` targets.
- Git pre-commit hook via `make setup` using `apdev-rs check-chars`.
- 671 tests across unit, integration, and doc-test suites.

### Changed
- `apcore` dependency switched from local path (`../apcore-rust`) to published crate (`apcore = "0.13"`).
- Explorer module refactored: hand-rolled `api.rs` and `templates.rs` replaced by `mcp-embedded-ui = "0.3"` crate with bridge adapters (`AuthBridge`, `wrap_call_fn`).
- `OutputFormatter` type alias uses `Box<dyn Fn>` (Send + Sync) for custom result formatting.
- `StreamResult` type alias introduced for `Pin<Box<dyn Stream<Item = Result<Value, ExecutorError>> + Send>>`.
- `ReadResourceHandler` type alias introduced for the read_resource handler closure.
- `ExecutionRouter::new_with_formatter()` constructor added for creating routers with pre-configured settings but no executor.

### Removed
- `src/explorer/api.rs` — ExplorerState, API handlers, and CallResponse (replaced by `mcp-embedded-ui`).
- `src/explorer/templates.rs` — HTML template rendering (replaced by `mcp-embedded-ui`).

[0.11.2]: https://github.com/aiperceivable/apcore-mcp-rust/compare/v0.11.1...v0.11.2
[0.11.1]: https://github.com/aiperceivable/apcore-mcp-rust/compare/v0.11.0...v0.11.1
[0.11.0]: https://github.com/aiperceivable/apcore-mcp-rust/compare/v0.10.1...v0.11.0
[0.10.1]: https://github.com/aiperceivable/apcore-mcp-rust/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/aiperceivable/apcore-mcp-rust/releases/tag/v0.10.0
