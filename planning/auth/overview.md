# Auth Feature — Overview

## Overview

Port the Python authentication module to idiomatic Rust. The module provides JWT-based authentication for MCP HTTP transports, consisting of an `Authenticator` trait, a `JWTAuthenticator` implementation, and tower-based `AuthMiddleware` that propagates identity via tokio task-locals.

## Scope

- Replace stub code in `src/auth/` with full implementations
- Align `Identity` type with `apcore::Identity` from the core crate
- Implement JWT decoding and validation using the `jsonwebtoken` crate
- Implement tower `Layer`/`Service` middleware with exempt paths, 401 responses, and task-local identity propagation
- Comprehensive unit and integration tests

**Out of scope:** RSA/EC key support (HMAC only in initial implementation), key rotation, token refresh.

## Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Language | Rust | edition 2021 |
| Async runtime | tokio | 1.x |
| Serialization | serde + serde_json | 1.x |
| JWT | jsonwebtoken | 9.x |
| HTTP framework | axum | 0.8.x |
| Middleware | tower | 0.5.x |
| Trait async | async_trait | 0.1.x |
| Logging | tracing | 0.1.x |
| Identity type | apcore::Identity | 0.13.x |

## Task Execution Order

| Order | Task ID | Title | Est. Time | Dependencies |
|-------|---------|-------|-----------|--------------|
| 1 | align-identity-type | Align protocol.rs with apcore::Identity | 30 min | none |
| 2 | implement-claim-mapping | Complete ClaimMapping with all fields | 30 min | align-identity-type |
| 3 | implement-jwt-authenticator | Implement JWTAuthenticator with jsonwebtoken | 1.5 hr | implement-claim-mapping |
| 4 | implement-tower-middleware | Implement tower Service for AuthMiddleware | 1.5 hr | align-identity-type, implement-jwt-authenticator |
| 5 | add-jwt-authenticator-tests | Unit tests for JWT authenticator | 1 hr | implement-jwt-authenticator |
| 6 | add-middleware-tests | Unit tests for tower middleware | 1 hr | implement-tower-middleware |
| 7 | add-integration-tests | End-to-end auth flow tests | 1 hr | add-jwt-authenticator-tests, add-middleware-tests |
| 8 | update-module-exports | Clean up mod.rs and remove stubs | 20 min | add-integration-tests |

**Note:** Tasks 4 and 5 can run in parallel after task 3 completes. Tasks 5 and 6 can also run in parallel.

## Progress

| Task ID | Status |
|---------|--------|
| align-identity-type | not started |
| implement-claim-mapping | not started |
| implement-jwt-authenticator | not started |
| implement-tower-middleware | not started |
| add-jwt-authenticator-tests | not started |
| add-middleware-tests | not started |
| add-integration-tests | not started |
| update-module-exports | not started |

## Reference Documents

- Feature spec: `docs/features/auth.md`
- Type mapping spec: `apcore/docs/spec/type-mapping.md`
- Python reference implementation: `apcore-mcp-python/src/apcore_mcp/auth/`
- Core Identity type: `apcore-rust/src/context.rs`
- Implementation plan: `planning/auth/plan.md`
