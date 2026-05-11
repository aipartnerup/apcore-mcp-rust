# Task: implement-jwt-authenticator

## Goal

Implement `JWTAuthenticator` that extracts a Bearer token from the Authorization header, decodes/validates it using the `jsonwebtoken` crate, and maps claims to `apcore::Identity` via `ClaimMapping`.

## Files Involved

- `src/auth/jwt.rs` — full implementation of `JWTAuthenticator`

## Steps (TDD-first)

1. **Write unit tests first** (see task `add-jwt-authenticator-tests` for details, but basic smoke tests here):
   - Valid HS256 token produces `Some(Identity)`.
   - Missing Authorization header returns `None`.
   - Malformed Bearer prefix returns `None`.
2. **Add full fields to `JWTAuthenticator`:**
   ```rust
   pub struct JWTAuthenticator {
       key: DecodingKey,
       algorithms: Vec<Algorithm>,
       audience: Option<String>,
       issuer: Option<String>,
       claim_mapping: ClaimMapping,
       require_claims: Vec<String>,
       require_auth: bool,
   }
   ```
3. **Implement `JWTAuthenticator::new()`** accepting:
   - `key: &str` — HMAC secret
   - Named parameters via builder or struct: `algorithms`, `audience`, `issuer`, `claim_mapping`, `require_claims`, `require_auth`
   - Defaults: `algorithms = [HS256]`, `require_claims = ["sub"]`, `require_auth = true`
4. **Implement `require_auth()` accessor** returning `bool`.
5. **Implement private `decode_token()`:**
   - Build `jsonwebtoken::Validation` with algorithms, audience, issuer.
   - Set `required_spec_claims` from `require_claims`.
   - Call `jsonwebtoken::decode::<HashMap<String, serde_json::Value>>()`.
   - Return `None` on any error (log with `tracing::debug!`).
6. **Implement private `payload_to_identity()`:**
   - Extract `id` from `claim_mapping.id_claim` — return `None` if missing.
   - Extract `identity_type` from `claim_mapping.type_claim` (default `"user"`).
   - Extract `roles` from `claim_mapping.roles_claim` (expect JSON array, default empty).
   - Extract `attrs` from `claim_mapping.attrs_claims` (copy named claims).
   - Construct and return `apcore::Identity`.
7. **Implement `Authenticator` trait:**
   - Extract `authorization` header (case-insensitive lookup).
   - Check `bearer ` prefix.
   - Strip and decode token.
   - Map to identity.
8. **Remove `#![allow(unused)]`** and all `todo!()` macros.
9. **Run `cargo check` and `cargo test`.**

## Acceptance Criteria

- [ ] Bearer token extraction is case-insensitive for "Bearer" prefix
- [ ] JWT validation uses configured algorithms, audience, issuer
- [ ] `require_claims` are enforced by the jsonwebtoken crate
- [ ] Claims are mapped to `apcore::Identity` fields correctly
- [ ] Missing id claim returns `None`
- [ ] Decoding errors are logged at debug level and return `None`
- [ ] `require_auth` accessor works
- [ ] Default algorithm is HS256

## Dependencies

- implement-claim-mapping

## Estimated Time

1.5 hours
