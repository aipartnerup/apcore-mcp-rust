# Feature: Authentication

## Module Purpose
Provides JWT-based authentication for MCP HTTP transports. Defines the Authenticator trait, JWT implementation, and HTTP middleware.

## Public API Surface

### Authenticator (trait)
- `async authenticate(headers) -> Option<Identity>`

### ClaimMapping
- Fields: id_claim, type_claim, roles_claim, attrs_claims

### JWTAuthenticator
- `new(key, algorithms, audience, issuer, claim_mapping, require_claims, require_auth) -> JWTAuthenticator`
- `async authenticate(headers) -> Option<Identity>`
- `require_auth() -> bool`

### AuthMiddleware (tower Layer)
- `new(app, authenticator, exempt_paths, exempt_prefixes, require_auth) -> AuthMiddleware`
- Default exempt paths: {"/health", "/metrics"}
- Sets AUTH_IDENTITY task-local for downstream handlers
- Returns 401 JSON on auth failure with WWW-Authenticate: Bearer header

### Utility
- `extract_headers(scope) -> HashMap<String, String>`
- `AUTH_IDENTITY` — tokio task-local for current identity

## Acceptance Criteria
- [ ] Authenticator trait is object-safe and async
- [ ] JWTAuthenticator extracts Bearer token from Authorization header
- [ ] JWTAuthenticator validates JWT with configured key and algorithms
- [ ] JWTAuthenticator maps claims to Identity using ClaimMapping
- [ ] Default algorithm is HS256
- [ ] Supports audience and issuer validation
- [ ] AuthMiddleware skips exempt paths (default: /health, /metrics)
- [ ] AuthMiddleware sets AUTH_IDENTITY for downstream use
- [ ] AuthMiddleware returns 401 with proper JSON error on failure
- [ ] When require_auth=false, missing token is allowed (identity=None)
- [ ] JWT key resolution: key param > env var fallback
