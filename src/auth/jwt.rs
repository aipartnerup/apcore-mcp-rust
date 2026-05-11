//! JWTAuthenticator — authenticates MCP requests using JSON Web Tokens.

use std::collections::HashMap;

use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::auth::protocol::{Authenticator, Identity};

/// Mapping from JWT claim names to Identity fields.
///
/// Mirrors the Python `ClaimMapping` dataclass, controlling which JWT claims
/// are read when constructing an [`Identity`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimMapping {
    /// JWT claim used as `Identity.id` (default: `"sub"`).
    pub id_claim: String,
    /// JWT claim used as `Identity.type` (default: `"type"`).
    pub type_claim: String,
    /// JWT claim used as `Identity.roles` — expects a list (default: `"roles"`).
    pub roles_claim: String,
    /// Extra claims to copy into `Identity.attrs` (default: `None`).
    pub attrs_claims: Option<Vec<String>>,
}

impl Default for ClaimMapping {
    fn default() -> Self {
        Self {
            id_claim: "sub".to_string(),
            type_claim: "type".to_string(),
            roles_claim: "roles".to_string(),
            attrs_claims: None,
        }
    }
}

/// JWT-based authenticator.
///
/// Validates JWT Bearer tokens from the `Authorization` header and maps
/// claims to [`Identity`] via [`ClaimMapping`].
pub struct JWTAuthenticator {
    key: DecodingKey,
    algorithms: Vec<Algorithm>,
    audience: Option<String>,
    issuer: Option<String>,
    claim_mapping: ClaimMapping,
    require_claims: Vec<String>,
    /// Whether unauthenticated requests should be rejected at this authenticator.
    ///
    /// NOTE [D1-004]: Python delegates this policy to `AuthMiddleware` rather than
    /// owning it on the authenticator. TypeScript and Rust own it on the authenticator
    /// itself (consistent with each other). This is a known cross-language asymmetry,
    /// documented here to aid future spec alignment.
    require_auth: bool,
}

impl JWTAuthenticator {
    /// Create a new JWT authenticator.
    ///
    /// # Arguments
    /// * `key` — HMAC secret (or symmetric key) for token verification.
    /// * `algorithms` — Allowed JWT algorithms. Defaults to `[HS256]`.
    /// * `audience` — Expected `aud` claim (optional).
    /// * `issuer` — Expected `iss` claim (optional).
    /// * `claim_mapping` — How JWT claims map to Identity fields. Defaults to [`ClaimMapping::default()`].
    /// * `require_claims` — Claims that must be present. Defaults to `["sub"]`.
    /// * `require_auth` — Whether authentication is mandatory. Defaults to `true`.
    pub fn new(
        key: &str,
        algorithms: Option<Vec<Algorithm>>,
        audience: Option<String>,
        issuer: Option<String>,
        claim_mapping: Option<ClaimMapping>,
        require_claims: Option<Vec<String>>,
        require_auth: Option<bool>,
    ) -> Self {
        Self {
            key: DecodingKey::from_secret(key.as_bytes()),
            algorithms: algorithms.unwrap_or_else(|| vec![Algorithm::HS256]),
            audience,
            issuer,
            claim_mapping: claim_mapping.unwrap_or_default(),
            require_claims: require_claims.unwrap_or_else(|| vec!["sub".to_string()]),
            require_auth: require_auth.unwrap_or(true),
        }
    }

    /// Whether unauthenticated requests should be rejected.
    pub fn require_auth(&self) -> bool {
        self.require_auth
    }

    /// Decode and validate a JWT token. Returns `None` on any error.
    fn decode_token(&self, token: &str) -> Option<HashMap<String, serde_json::Value>> {
        let mut validation = Validation::new(self.algorithms[0]);
        if self.algorithms.len() > 1 {
            validation.algorithms = self.algorithms.clone();
        }
        // [JWT-3] Spec mandates ~30s clock-skew leeway. jsonwebtoken's
        // default Validation::leeway is 60s; align to 30s for parity
        // with Python+TS post-fix and the documented value.
        validation.leeway = 30;

        // Configure audience validation
        match &self.audience {
            Some(aud) => validation.set_audience(&[aud]),
            None => {
                validation.validate_aud = false;
            }
        }

        // Configure issuer validation
        if let Some(iss) = &self.issuer {
            validation.set_issuer(&[iss]);
        }

        // Configure required claims
        validation.set_required_spec_claims(
            &self
                .require_claims
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );

        match jsonwebtoken::decode::<HashMap<String, serde_json::Value>>(
            token,
            &self.key,
            &validation,
        ) {
            Ok(token_data) => {
                let claims = token_data.claims;
                // [D11-006] Post-decode custom-claim enforcement.
                // `set_required_spec_claims` only accepts known JWT spec claims
                // (exp/nbf/iat/aud/iss/sub). Custom claims like "org_id" are
                // silently ignored by jsonwebtoken. We enforce them here,
                // matching Python+TS post-decode loops.
                for required in &self.require_claims {
                    // Only check claims not already validated by jsonwebtoken
                    // (standard spec claims are handled above).
                    const SPEC_CLAIMS: &[&str] = &["exp", "nbf", "iat", "aud", "iss", "sub"];
                    if SPEC_CLAIMS.contains(&required.as_str()) {
                        // Already enforced by jsonwebtoken above.
                        continue;
                    }
                    if !claims.contains_key(required) {
                        debug!("JWT missing required custom claim: {required}");
                        return None;
                    }
                }
                Some(claims)
            }
            Err(e) => {
                debug!("JWT validation failed: {e}");
                None
            }
        }
    }

    /// Convert a decoded JWT payload to an [`Identity`].
    fn payload_to_identity(
        &self,
        payload: &HashMap<String, serde_json::Value>,
    ) -> Option<Identity> {
        let mapping = &self.claim_mapping;

        // Extract id — required
        let id = payload.get(&mapping.id_claim)?;
        let id = match id {
            serde_json::Value::String(s) => s.clone(),
            // [D11-008] JSON null must short-circuit to None (no identity),
            // not produce the string literal "null". Python short-circuits on
            // None; TS checks rawId === null. Rust now aligns.
            serde_json::Value::Null => return None,
            other => other.to_string(),
        };

        // Extract identity_type — default "user".
        // [D11-107] Treat an empty string the same as a missing claim so
        // Rust falls back to "user". Matches Python's `payload.get(...) or
        // "user"` semantics (where "" is falsy) and TypeScript's
        // truthy-check fallback.
        let identity_type = payload
            .get(&mapping.type_claim)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("user")
            .to_string();

        // Extract roles — expects JSON array, default empty
        let roles = payload
            .get(&mapping.roles_claim)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Extract attrs from attrs_claims
        let mut attrs = HashMap::new();
        if let Some(claims) = &mapping.attrs_claims {
            for claim in claims {
                if let Some(value) = payload.get(claim) {
                    attrs.insert(claim.clone(), value.clone());
                }
            }
        }

        Some(Identity::new(id, identity_type, roles, attrs))
    }
}

#[async_trait]
impl Authenticator for JWTAuthenticator {
    async fn authenticate(&self, headers: &HashMap<String, String>) -> Option<Identity> {
        // Case-insensitive lookup for "authorization" header
        let auth_header = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .map(|(_, v)| v.as_str())?;

        // Check for "Bearer " prefix (case-insensitive)
        if auth_header.len() < 7 || !auth_header[..7].eq_ignore_ascii_case("bearer ") {
            return None;
        }

        let token = auth_header[7..].trim();
        if token.is_empty() {
            return None;
        }

        let payload = self.decode_token(token)?;
        self.payload_to_identity(&payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header};

    // ── ClaimMapping tests (pre-existing) ──────────────────────────────

    #[test]
    fn claim_mapping_default_id_claim() {
        let mapping = ClaimMapping::default();
        assert_eq!(mapping.id_claim, "sub");
    }

    #[test]
    fn claim_mapping_default_type_claim() {
        let mapping = ClaimMapping::default();
        assert_eq!(mapping.type_claim, "type");
    }

    #[test]
    fn claim_mapping_default_roles_claim() {
        let mapping = ClaimMapping::default();
        assert_eq!(mapping.roles_claim, "roles");
    }

    #[test]
    fn claim_mapping_default_attrs_claims_is_none() {
        let mapping = ClaimMapping::default();
        assert!(mapping.attrs_claims.is_none());
    }

    #[test]
    fn claim_mapping_custom_values() {
        let mapping = ClaimMapping {
            id_claim: "user_id".to_string(),
            type_claim: "kind".to_string(),
            roles_claim: "permissions".to_string(),
            attrs_claims: Some(vec!["email".to_string(), "org".to_string()]),
        };
        assert_eq!(mapping.id_claim, "user_id");
        assert_eq!(mapping.type_claim, "kind");
        assert_eq!(mapping.roles_claim, "permissions");
        assert_eq!(mapping.attrs_claims.as_ref().unwrap().len(), 2);
        assert_eq!(mapping.attrs_claims.as_ref().unwrap()[0], "email");
        assert_eq!(mapping.attrs_claims.as_ref().unwrap()[1], "org");
    }

    #[test]
    fn claim_mapping_clone() {
        let original = ClaimMapping::default();
        let cloned = original.clone();
        assert_eq!(cloned.id_claim, original.id_claim);
        assert_eq!(cloned.type_claim, original.type_claim);
        assert_eq!(cloned.roles_claim, original.roles_claim);
        assert_eq!(cloned.attrs_claims, original.attrs_claims);
    }

    #[test]
    fn claim_mapping_debug() {
        let mapping = ClaimMapping::default();
        let debug_str = format!("{:?}", mapping);
        assert!(debug_str.contains("ClaimMapping"));
        assert!(debug_str.contains("sub"));
    }

    #[test]
    fn claim_mapping_serialize_deserialize() {
        let original = ClaimMapping {
            id_claim: "sub".to_string(),
            type_claim: "type".to_string(),
            roles_claim: "roles".to_string(),
            attrs_claims: Some(vec!["email".to_string()]),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: ClaimMapping = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.id_claim, original.id_claim);
        assert_eq!(restored.type_claim, original.type_claim);
        assert_eq!(restored.roles_claim, original.roles_claim);
        assert_eq!(restored.attrs_claims, original.attrs_claims);
    }

    // ── Helper ─────────────────────────────────────────────────────────

    const TEST_SECRET: &str = "super-secret-key-for-testing";

    fn make_token(claims: &serde_json::Value) -> String {
        jsonwebtoken::encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .expect("encode JWT")
    }

    fn make_authenticator() -> JWTAuthenticator {
        JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, None)
    }

    fn headers_with_token(token: &str) -> HashMap<String, String> {
        let mut h = HashMap::new();
        h.insert("authorization".to_string(), format!("Bearer {token}"));
        h
    }

    // ── JWTAuthenticator constructor tests ─────────────────────────────

    #[test]
    fn new_defaults_algorithm_to_hs256() {
        let auth = make_authenticator();
        assert_eq!(auth.algorithms, vec![Algorithm::HS256]);
    }

    #[test]
    fn new_defaults_require_auth_to_true() {
        let auth = make_authenticator();
        assert!(auth.require_auth());
    }

    #[test]
    fn new_defaults_require_claims_to_sub() {
        let auth = make_authenticator();
        assert_eq!(auth.require_claims, vec!["sub".to_string()]);
    }

    #[test]
    fn new_custom_require_auth_false() {
        let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, Some(false));
        assert!(!auth.require_auth());
    }

    // ── authenticate tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn valid_token_produces_identity() {
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({
            "sub": "user-42",
            "type": "human",
            "roles": ["admin", "reader"],
        }));
        let headers = headers_with_token(&token);
        let identity = auth
            .authenticate(&headers)
            .await
            .expect("should authenticate");
        assert_eq!(identity.id(), "user-42");
        assert_eq!(identity.identity_type(), "human");
        assert_eq!(identity.roles(), vec!["admin", "reader"]);
    }

    #[tokio::test]
    async fn missing_authorization_header_returns_none() {
        let auth = make_authenticator();
        let result = auth.authenticate(&HashMap::new()).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn malformed_bearer_prefix_returns_none() {
        let auth = make_authenticator();
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), "Basic abc123".to_string());
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn empty_bearer_token_returns_none() {
        let auth = make_authenticator();
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), "Bearer ".to_string());
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn invalid_jwt_returns_none() {
        let auth = make_authenticator();
        let headers = headers_with_token("not-a-real-jwt");
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn wrong_secret_returns_none() {
        let token = jsonwebtoken::encode(
            &Header::default(),
            &serde_json::json!({"sub": "user-1"}),
            &EncodingKey::from_secret(b"wrong-secret"),
        )
        .unwrap();
        let auth = make_authenticator();
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn missing_id_claim_returns_none() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            None,
            Some(vec![]), // don't require "sub" so decode succeeds
            None,
        );
        let token = make_token(&serde_json::json!({"type": "service"}));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn default_identity_type_is_user() {
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({"sub": "user-1"}));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await.unwrap();
        assert_eq!(identity.identity_type(), "user");
    }

    #[tokio::test]
    async fn empty_type_claim_falls_back_to_user() {
        // [D11-107] An empty `type` claim must be treated as missing so
        // Rust falls back to the default "user", matching Python's
        // `payload.get(...) or "user"` (where "" is falsy) and the
        // truthy-check fallback used in TypeScript.
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({
            "sub": "user-1",
            "type": "",
        }));
        let headers = headers_with_token(&token);
        let identity = auth
            .authenticate(&headers)
            .await
            .expect("auth should succeed with empty type claim");
        assert_eq!(
            identity.identity_type(),
            "user",
            "empty `type` claim must fall back to 'user'"
        );
    }

    #[tokio::test]
    async fn default_roles_is_empty() {
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({"sub": "user-1"}));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await.unwrap();
        assert!(identity.roles().is_empty());
    }

    #[tokio::test]
    async fn attrs_claims_are_extracted() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            Some(ClaimMapping {
                attrs_claims: Some(vec!["email".to_string(), "org".to_string()]),
                ..ClaimMapping::default()
            }),
            None,
            None,
        );
        let token = make_token(&serde_json::json!({
            "sub": "user-1",
            "email": "alice@example.com",
            "org": "acme",
        }));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await.unwrap();
        assert_eq!(
            identity.attrs().get("email").and_then(|v| v.as_str()),
            Some("alice@example.com")
        );
        assert_eq!(
            identity.attrs().get("org").and_then(|v| v.as_str()),
            Some("acme")
        );
    }

    #[tokio::test]
    async fn missing_attrs_claim_is_skipped() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            Some(ClaimMapping {
                attrs_claims: Some(vec!["email".to_string()]),
                ..ClaimMapping::default()
            }),
            None,
            None,
        );
        let token = make_token(&serde_json::json!({"sub": "user-1"}));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await.unwrap();
        assert!(identity.attrs().is_empty());
    }

    #[tokio::test]
    async fn case_insensitive_authorization_header() {
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({"sub": "user-1"}));
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {token}"));
        let identity = auth.authenticate(&headers).await;
        assert!(identity.is_some());
    }

    #[tokio::test]
    async fn case_insensitive_bearer_prefix() {
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({"sub": "user-1"}));
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), format!("bearer {token}"));
        let identity = auth.authenticate(&headers).await;
        assert!(identity.is_some());
    }

    #[tokio::test]
    async fn audience_validation_rejects_wrong_aud() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            Some("my-api".to_string()),
            None,
            None,
            None,
            None,
        );
        let token = make_token(&serde_json::json!({"sub": "user-1", "aud": "other-api"}));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn audience_validation_accepts_correct_aud() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            Some("my-api".to_string()),
            None,
            None,
            None,
            None,
        );
        let token = make_token(&serde_json::json!({"sub": "user-1", "aud": "my-api"}));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await;
        assert!(identity.is_some());
    }

    #[tokio::test]
    async fn issuer_validation_rejects_wrong_iss() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            Some("trusted-issuer".to_string()),
            None,
            None,
            None,
        );
        let token = make_token(&serde_json::json!({"sub": "user-1", "iss": "untrusted"}));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn issuer_validation_accepts_correct_iss() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            Some("trusted-issuer".to_string()),
            None,
            None,
            None,
        );
        let token = make_token(&serde_json::json!({"sub": "user-1", "iss": "trusted-issuer"}));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await;
        assert!(identity.is_some());
    }

    #[tokio::test]
    async fn require_claims_enforced() {
        // Require "sub" (default) — token without sub should fail
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({"name": "alice"}));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn expired_token_returns_none() {
        let auth = make_authenticator();
        let token = make_token(&serde_json::json!({
            "sub": "user-1",
            "exp": 0, // epoch — long expired
        }));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(result.is_none());
    }

    // -- Issue D11-006: custom require_claims enforced post-decode ----------

    #[tokio::test]
    async fn require_custom_claim_absent_returns_none() {
        // [D11-006] Custom claim "org_id" is not enforced by jsonwebtoken's
        // set_required_spec_claims. Post-decode loop must reject tokens lacking it.
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            None,
            Some(vec!["sub".to_string(), "org_id".to_string()]),
            None,
        );
        // Token WITHOUT org_id
        let token = make_token(&serde_json::json!({"sub": "user-1"}));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(
            result.is_none(),
            "should reject token missing required custom claim 'org_id'"
        );
    }

    #[tokio::test]
    async fn require_custom_claim_present_succeeds() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            None,
            Some(vec!["sub".to_string(), "org_id".to_string()]),
            None,
        );
        let token = make_token(&serde_json::json!({"sub": "user-1", "org_id": "acme"}));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(
            result.is_some(),
            "should accept token with all required claims"
        );
    }

    // -- Issue D11-008: JSON null sub → None, not "null" string ---------------

    #[tokio::test]
    async fn null_sub_claim_returns_none() {
        // [D11-008] sub: null must return None, not Some(Identity{id: "null"}).
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            None,
            Some(vec![]), // don't require "sub" so decode succeeds with null sub
            None,
        );
        let token = make_token(&serde_json::json!({"sub": null}));
        let headers = headers_with_token(&token);
        let result = auth.authenticate(&headers).await;
        assert!(
            result.is_none(),
            "null sub claim must return None, not Some(Identity{{id: \"null\"}})"
        );
    }

    // -- Issue D11-009: null identity_type → "user" (documented behavior) ----

    #[tokio::test]
    async fn null_identity_type_falls_back_to_user() {
        // [D11-009] type: null → identity_type == "user". Rust's
        // .and_then(as_str).unwrap_or("user") handles null correctly since
        // null.as_str() == None → unwrap_or("user"). This is the most
        // defensive behavior and is intentional.
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            None,
            Some(vec![]), // no required claims
            None,
        );
        let token = make_token(&serde_json::json!({"sub": "user-1", "type": null}));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await.unwrap();
        assert_eq!(
            identity.identity_type(),
            "user",
            "null type claim must fall back to 'user'"
        );
    }

    // -- Issue D1-004: require_auth accessor (documented asymmetry) -----------

    #[test]
    fn require_auth_accessor_returns_value() {
        // [D1-004] Rust owns require_auth on the authenticator (consistent with TS).
        // Python delegates this policy to AuthMiddleware instead.
        let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, Some(true));
        assert!(
            auth.require_auth(),
            "require_auth() must return true when set to true"
        );
        let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, Some(false));
        assert!(
            !auth.require_auth(),
            "require_auth() must return false when set to false"
        );
    }

    #[tokio::test]
    async fn custom_claim_mapping() {
        let auth = JWTAuthenticator::new(
            TEST_SECRET,
            None,
            None,
            None,
            Some(ClaimMapping {
                id_claim: "user_id".to_string(),
                type_claim: "kind".to_string(),
                roles_claim: "permissions".to_string(),
                attrs_claims: None,
            }),
            Some(vec!["user_id".to_string()]),
            None,
        );
        let token = make_token(&serde_json::json!({
            "user_id": "u-99",
            "kind": "service",
            "permissions": ["write"],
        }));
        let headers = headers_with_token(&token);
        let identity = auth.authenticate(&headers).await.unwrap();
        assert_eq!(identity.id(), "u-99");
        assert_eq!(identity.identity_type(), "service");
        assert_eq!(identity.roles(), vec!["write"]);
    }
}
