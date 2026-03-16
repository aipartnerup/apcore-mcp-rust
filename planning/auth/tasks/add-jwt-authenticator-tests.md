# Task: add-jwt-authenticator-tests

## Goal

Write comprehensive unit tests for `JWTAuthenticator` covering all authentication scenarios.

## Files Involved

- `src/auth/jwt.rs` — add `#[cfg(test)] mod tests` block

## Steps (TDD-first)

1. **Create test helper** `make_token(claims, secret)` that encodes a JWT with HS256.
2. **Create test helper** `make_headers(token)` that builds a `HashMap` with `authorization: Bearer {token}`.
3. **Write tests:**
   - `test_valid_token_returns_identity` — encode a token with `sub`, `type`, `roles`, verify `Identity` fields.
   - `test_missing_auth_header_returns_none` — empty headers.
   - `test_non_bearer_auth_returns_none` — `authorization: Basic ...`.
   - `test_empty_bearer_token_returns_none` — `authorization: Bearer `.
   - `test_invalid_token_returns_none` — garbled string.
   - `test_expired_token_returns_none` — token with `exp` in the past.
   - `test_wrong_secret_returns_none` — decode with different key.
   - `test_missing_sub_claim_returns_none` — token without `sub`.
   - `test_audience_validation` — token with wrong `aud` fails; correct `aud` succeeds.
   - `test_issuer_validation` — token with wrong `iss` fails; correct `iss` succeeds.
   - `test_custom_claim_mapping` — use non-default claim names.
   - `test_attrs_claims_extraction` — verify attrs_claims copies specified claims into `Identity.attrs`.
   - `test_roles_defaults_to_empty` — token without roles claim produces empty roles vec.
   - `test_type_defaults_to_user` — token without type claim produces `"user"`.
   - `test_require_auth_accessor` — verify `require_auth()` returns configured value.

## Acceptance Criteria

- [ ] All listed tests are implemented and pass
- [ ] Tests use the `jsonwebtoken` crate to create test tokens (no external dependencies)
- [ ] Tests cover both success and failure paths
- [ ] Tests verify `Identity` field values, not just `Some`/`None`

## Dependencies

- implement-jwt-authenticator

## Estimated Time

1 hour
