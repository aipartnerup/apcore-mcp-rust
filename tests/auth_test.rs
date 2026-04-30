//! Integration tests for the auth module.

mod common;

use apcore_mcp::auth::protocol::Authenticator;
use apcore_mcp::{ClaimMapping, JWTAuthenticator};
use jsonwebtoken::{EncodingKey, Header};
use std::collections::HashMap;

const TEST_SECRET: &str = "integration-test-secret";

fn make_token(claims: &serde_json::Value) -> String {
    jsonwebtoken::encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
    )
    .expect("encode JWT")
}

fn headers_with_token(token: &str) -> HashMap<String, String> {
    let mut h = HashMap::new();
    h.insert("authorization".to_string(), format!("Bearer {token}"));
    h
}

// ---- JWTAuthenticator with valid tokens ------------------------------------

#[tokio::test]
async fn valid_token_produces_identity() {
    let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, None);
    let token = make_token(&serde_json::json!({
        "sub": "alice",
        "type": "human",
        "roles": ["admin"]
    }));
    let identity = auth
        .authenticate(&headers_with_token(&token))
        .await
        .expect("valid token must produce identity");
    assert_eq!(identity.id(), "alice");
    assert_eq!(identity.identity_type(), "human");
    assert_eq!(identity.roles(), vec!["admin"]);
}

#[tokio::test]
async fn invalid_token_returns_none() {
    let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, None);
    let result = auth
        .authenticate(&headers_with_token("not.a.valid.jwt"))
        .await;
    assert!(result.is_none(), "invalid JWT must return None");
}

// ---- require_claims enforcement --------------------------------------------

#[tokio::test]
async fn missing_custom_claim_returns_none() {
    // [D11-006] Custom claims must be enforced post-decode.
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
    let result = auth.authenticate(&headers_with_token(&token)).await;
    assert!(result.is_none(), "missing custom claim must reject");
}

#[tokio::test]
async fn present_custom_claim_returns_identity() {
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
    let result = auth.authenticate(&headers_with_token(&token)).await;
    assert!(result.is_some(), "all required claims present must succeed");
}

// ---- Null sub → None [D11-008] --------------------------------------------

#[tokio::test]
async fn null_sub_returns_none_not_null_string() {
    // [D11-008] JSON null sub must return None, not Some(Identity{id:"null"}).
    let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, Some(vec![]), None);
    let token = make_token(&serde_json::json!({"sub": null}));
    let result = auth.authenticate(&headers_with_token(&token)).await;
    assert!(result.is_none(), "null sub must return None");
}

// ---- ClaimMapping custom fields -------------------------------------------

#[tokio::test]
async fn custom_claim_mapping_works() {
    let auth = JWTAuthenticator::new(
        TEST_SECRET,
        None,
        None,
        None,
        Some(ClaimMapping {
            id_claim: "user_id".to_string(),
            type_claim: "kind".to_string(),
            roles_claim: "perms".to_string(),
            attrs_claims: None,
        }),
        Some(vec!["user_id".to_string()]),
        None,
    );
    let token = make_token(&serde_json::json!({
        "user_id": "u-99",
        "kind": "service",
        "perms": ["write", "read"]
    }));
    let identity = auth
        .authenticate(&headers_with_token(&token))
        .await
        .unwrap();
    assert_eq!(identity.id(), "u-99");
    assert_eq!(identity.identity_type(), "service");
    assert!(identity.roles().contains(&"write".to_string()));
}

// ---- require_auth accessor [D1-004] ----------------------------------------

#[test]
fn require_auth_accessor() {
    let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, Some(true));
    assert!(auth.require_auth());
    let auth = JWTAuthenticator::new(TEST_SECRET, None, None, None, None, None, Some(false));
    assert!(!auth.require_auth());
}
