//! Auth sub-module — authentication and authorization for MCP transports.
//!
//! Provides JWT-based authentication, a Tower middleware layer for protecting
//! HTTP endpoints, and task-local identity propagation.

pub mod jwt;
pub mod middleware;
pub mod protocol;

// ---- Re-exports: public API surface -----------------------------------------
pub use protocol::{Authenticator, Identity};
pub use jwt::{ClaimMapping, JWTAuthenticator};
pub use middleware::{AuthMiddlewareLayer, AuthMiddlewareService, AUTH_IDENTITY, extract_headers};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashSet;
    use std::convert::Infallible;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, Response, StatusCode};
    use jsonwebtoken::{EncodingKey, Header};
    use tower::{Service, ServiceBuilder, ServiceExt};

    const SECRET: &str = "integration-test-secret";

    fn make_jwt(claims: &serde_json::Value) -> String {
        jsonwebtoken::encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("encode JWT")
    }

    /// Inner service that reads the task-local identity and returns it as JSON.
    async fn whoami_handler(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let body = AUTH_IDENTITY
            .try_with(|id| match id {
                Some(identity) => serde_json::to_string(identity).unwrap(),
                None => r#"{"anonymous":true}"#.to_string(),
            })
            .unwrap_or_else(|_| r#"{"error":"no task-local"}"#.to_string());

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap())
    }

    fn build_stack(
        require_auth: bool,
    ) -> impl Service<Request<Body>, Response = Response<Body>, Error = Infallible> + Clone {
        let authenticator: Arc<dyn Authenticator> = Arc::new(JWTAuthenticator::new(
            SECRET, None, None, None, None, None, Some(require_auth),
        ));
        ServiceBuilder::new()
            .layer(AuthMiddlewareLayer::new(authenticator).require_auth(require_auth))
            .service(tower::service_fn(whoami_handler))
    }

    #[tokio::test]
    async fn full_flow_valid_jwt() {
        let svc = build_stack(true);
        let token = make_jwt(&serde_json::json!({
            "sub": "alice",
            "type": "human",
            "roles": ["admin"],
        }));

        let req = Request::get("/api/whoami")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], "alice");
        assert_eq!(json["type"], "human");
        assert_eq!(json["roles"][0], "admin");
    }

    #[tokio::test]
    async fn full_flow_no_token_returns_401() {
        let svc = build_stack(true);

        let req = Request::get("/api/whoami")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Unauthorized");
    }

    #[tokio::test]
    async fn full_flow_exempt_health() {
        let svc = build_stack(true);

        let req = Request::get("/health")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["anonymous"], true);
    }

    #[tokio::test]
    async fn full_flow_permissive_mode() {
        let svc = build_stack(false);

        let req = Request::get("/api/whoami")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["anonymous"], true);
    }

    #[tokio::test]
    async fn full_flow_invalid_jwt_returns_401() {
        let svc = build_stack(true);

        let req = Request::get("/api/whoami")
            .header("Authorization", "Bearer not-a-valid-jwt")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn full_flow_custom_exempt_paths() {
        let authenticator: Arc<dyn Authenticator> = Arc::new(JWTAuthenticator::new(
            SECRET, None, None, None, None, None, Some(true),
        ));
        let layer = AuthMiddlewareLayer::new(authenticator)
            .exempt_paths(HashSet::from(["/status".to_string()]));
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(whoami_handler));

        // /status should be exempt
        let req = Request::get("/status").body(Body::empty()).unwrap();
        let resp = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // /health should NOT be exempt (defaults overridden)
        let req = Request::get("/health").body(Body::empty()).unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
