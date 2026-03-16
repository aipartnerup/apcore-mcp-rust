//! AuthMiddleware — tower middleware for authenticating MCP HTTP requests.
//!
//! Uses a task-local to propagate the authenticated identity through the
//! request handling pipeline.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use tower::{Layer, Service};
use tracing::warn;

use crate::auth::protocol::{Authenticator, Identity};

tokio::task_local! {
    /// Task-local storage for the authenticated identity.
    pub static AUTH_IDENTITY: Option<Identity>;
}

/// Extract headers from an HTTP request into a plain HashMap.
pub fn extract_headers<B>(req: &Request<B>) -> HashMap<String, String> {
    req.headers()
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|v| (name.to_string(), v.to_string()))
        })
        .collect()
}

/// Build a 401 Unauthorized JSON response.
fn build_401_response() -> Response<Body> {
    let body = serde_json::json!({
        "error": "Unauthorized",
        "detail": "Missing or invalid Bearer token"
    });

    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("content-type", "application/json")
        .header("www-authenticate", "Bearer")
        .body(Body::from(body.to_string()))
        .expect("failed to build 401 response")
}

/// Tower [`Layer`] that wraps services with authentication.
#[derive(Clone)]
pub struct AuthMiddlewareLayer {
    authenticator: Arc<dyn Authenticator>,
    exempt_paths: HashSet<String>,
    exempt_prefixes: Vec<String>,
    require_auth: bool,
}

impl AuthMiddlewareLayer {
    /// Create a new auth middleware layer with default settings.
    ///
    /// Defaults:
    /// - `exempt_paths`: `{"/health", "/metrics"}`
    /// - `exempt_prefixes`: empty
    /// - `require_auth`: `true`
    pub fn new(authenticator: Arc<dyn Authenticator>) -> Self {
        Self {
            authenticator,
            exempt_paths: HashSet::from(["/health".to_string(), "/metrics".to_string()]),
            exempt_prefixes: Vec::new(),
            require_auth: true,
        }
    }

    /// Set the exact paths that bypass authentication.
    pub fn exempt_paths(mut self, paths: HashSet<String>) -> Self {
        self.exempt_paths = paths;
        self
    }

    /// Set the path prefixes that bypass authentication.
    pub fn exempt_prefixes(mut self, prefixes: Vec<String>) -> Self {
        self.exempt_prefixes = prefixes;
        self
    }

    /// Set whether authentication is required for non-exempt paths.
    ///
    /// When `false`, unauthenticated requests proceed with `None` identity.
    pub fn require_auth(mut self, require: bool) -> Self {
        self.require_auth = require;
        self
    }
}

impl<S> Layer<S> for AuthMiddlewareLayer {
    type Service = AuthMiddlewareService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddlewareService {
            inner,
            authenticator: self.authenticator.clone(),
            exempt_paths: self.exempt_paths.clone(),
            exempt_prefixes: self.exempt_prefixes.clone(),
            require_auth: self.require_auth,
        }
    }
}

/// Tower [`Service`] that authenticates requests before forwarding them.
#[derive(Clone)]
pub struct AuthMiddlewareService<S> {
    inner: S,
    authenticator: Arc<dyn Authenticator>,
    exempt_paths: HashSet<String>,
    exempt_prefixes: Vec<String>,
    require_auth: bool,
}

/// Check if a request path is exempt from authentication.
fn is_path_exempt(path: &str, exempt_paths: &HashSet<String>, exempt_prefixes: &[String]) -> bool {
    if exempt_paths.contains(path) {
        return true;
    }
    exempt_prefixes.iter().any(|prefix| path.starts_with(prefix))
}

impl<S> Service<Request<Body>> for AuthMiddlewareService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let authenticator = self.authenticator.clone();
        let exempt_paths = self.exempt_paths.clone();
        let exempt_prefixes = self.exempt_prefixes.clone();
        let require_auth = self.require_auth;
        // Clone inner service (tower best practice: use the ready service,
        // swap in a fresh clone for subsequent calls).
        let mut inner = self.inner.clone();
        std::mem::swap(&mut self.inner, &mut inner);

        Box::pin(async move {
            let path = req.uri().path().to_string();
            let is_exempt = is_path_exempt(&path, &exempt_paths, &exempt_prefixes);

            let headers = extract_headers(&req);

            if is_exempt {
                // Best-effort: try to authenticate but ignore failures.
                let identity = authenticator.authenticate(&headers).await;
                AUTH_IDENTITY
                    .scope(identity, inner.call(req))
                    .await
            } else {
                let identity = authenticator.authenticate(&headers).await;

                if identity.is_none() && require_auth {
                    warn!("Authentication failed for {}", path);
                    return Ok(build_401_response());
                }

                AUTH_IDENTITY
                    .scope(identity, inner.call(req))
                    .await
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::convert::Infallible;
    use tower::{ServiceBuilder, ServiceExt};

    /// A mock authenticator that accepts requests with a specific Bearer token.
    #[derive(Clone)]
    struct MockAuthenticator {
        valid_token: String,
    }

    #[async_trait]
    impl Authenticator for MockAuthenticator {
        async fn authenticate(&self, headers: &HashMap<String, String>) -> Option<Identity> {
            let auth = headers.get("authorization")?;
            let token = auth.strip_prefix("Bearer ")?;
            if token == self.valid_token {
                Some(Identity {
                    id: "test-user".to_string(),
                    identity_type: "human".to_string(),
                    roles: vec!["user".to_string()],
                    attrs: Default::default(),
                })
            } else {
                None
            }
        }
    }

    /// A mock authenticator that always rejects.
    struct RejectAuthenticator;

    #[async_trait]
    impl Authenticator for RejectAuthenticator {
        async fn authenticate(&self, _headers: &HashMap<String, String>) -> Option<Identity> {
            None
        }
    }

    /// A simple echo service that returns 200 with the identity id (or "anonymous").
    async fn echo_handler(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let identity_id = AUTH_IDENTITY
            .try_with(|id| {
                id.as_ref().map(|i| i.id.clone()).unwrap_or_else(|| "anonymous".to_string())
            })
            .unwrap_or_else(|_| "no-task-local".to_string());

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(identity_id))
            .unwrap())
    }

    fn make_service(
        authenticator: Arc<dyn Authenticator>,
    ) -> impl Service<Request<Body>, Response = Response<Body>, Error = Infallible> + Clone {
        ServiceBuilder::new()
            .layer(AuthMiddlewareLayer::new(authenticator))
            .service(tower::service_fn(echo_handler))
    }

    fn make_service_with_layer(
        layer: AuthMiddlewareLayer,
    ) -> impl Service<Request<Body>, Response = Response<Body>, Error = Infallible> + Clone {
        ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(echo_handler))
    }

    // ---- extract_headers tests ----

    #[test]
    fn extract_headers_basic() {
        let req = Request::builder()
            .header("Authorization", "Bearer abc123")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let headers = extract_headers(&req);
        assert_eq!(headers.get("authorization").unwrap(), "Bearer abc123");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn extract_headers_lowercases_names() {
        let req = Request::builder()
            .header("X-Custom-Header", "value")
            .body(Body::empty())
            .unwrap();

        let headers = extract_headers(&req);
        assert!(headers.contains_key("x-custom-header"));
        assert!(!headers.contains_key("X-Custom-Header"));
    }

    #[test]
    fn extract_headers_empty() {
        let req = Request::get("/")
            .body(Body::empty())
            .unwrap();
        let headers = extract_headers(&req);
        // Host may or may not be present depending on builder; just check no panic.
        assert!(headers.len() <= 1);
    }

    // ---- 401 response tests ----

    #[tokio::test]
    async fn unauthenticated_request_returns_401() {
        let auth: Arc<dyn Authenticator> = Arc::new(RejectAuthenticator);
        let svc = make_service(auth);

        let req = Request::get("/api/data")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            resp.headers().get("www-authenticate").unwrap(),
            "Bearer"
        );
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Unauthorized");
        assert_eq!(json["detail"], "Missing or invalid Bearer token");
    }

    // ---- authenticated request tests ----

    #[tokio::test]
    async fn authenticated_request_passes_through() {
        let auth: Arc<dyn Authenticator> = Arc::new(MockAuthenticator {
            valid_token: "secret".to_string(),
        });
        let svc = make_service(auth);

        let req = Request::get("/api/data")
            .header("Authorization", "Bearer secret")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"test-user");
    }

    // ---- exempt paths ----

    #[tokio::test]
    async fn exempt_path_bypasses_auth() {
        let auth: Arc<dyn Authenticator> = Arc::new(RejectAuthenticator);
        let svc = make_service(auth);

        let req = Request::get("/health")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"anonymous");
    }

    #[tokio::test]
    async fn metrics_path_is_exempt_by_default() {
        let auth: Arc<dyn Authenticator> = Arc::new(RejectAuthenticator);
        let svc = make_service(auth);

        let req = Request::get("/metrics")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn exempt_path_with_valid_token_populates_identity() {
        let auth: Arc<dyn Authenticator> = Arc::new(MockAuthenticator {
            valid_token: "secret".to_string(),
        });
        let svc = make_service(auth);

        let req = Request::get("/health")
            .header("Authorization", "Bearer secret")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"test-user");
    }

    #[tokio::test]
    async fn exempt_path_with_invalid_token_still_forwards() {
        let auth: Arc<dyn Authenticator> = Arc::new(MockAuthenticator {
            valid_token: "correct".to_string(),
        });
        let svc = make_service(auth);

        let req = Request::get("/health")
            .header("Authorization", "Bearer wrong-token")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        // Identity should be None since token was invalid, so we get "anonymous"
        assert_eq!(&body[..], b"anonymous");
    }

    // ---- exempt prefixes ----

    #[tokio::test]
    async fn exempt_prefix_bypasses_auth() {
        let auth: Arc<dyn Authenticator> = Arc::new(RejectAuthenticator);
        let layer = AuthMiddlewareLayer::new(auth)
            .exempt_prefixes(vec!["/public/".to_string()]);
        let svc = make_service_with_layer(layer);

        let req = Request::get("/public/docs/readme")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn non_matching_prefix_still_requires_auth() {
        let auth: Arc<dyn Authenticator> = Arc::new(RejectAuthenticator);
        let layer = AuthMiddlewareLayer::new(auth)
            .exempt_prefixes(vec!["/public/".to_string()]);
        let svc = make_service_with_layer(layer);

        let req = Request::get("/api/data")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ---- require_auth=false ----

    #[tokio::test]
    async fn require_auth_false_allows_unauthenticated() {
        let auth: Arc<dyn Authenticator> = Arc::new(RejectAuthenticator);
        let layer = AuthMiddlewareLayer::new(auth).require_auth(false);
        let svc = make_service_with_layer(layer);

        let req = Request::get("/api/data")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"anonymous");
    }

    #[tokio::test]
    async fn require_auth_false_still_populates_identity_when_valid() {
        let auth: Arc<dyn Authenticator> = Arc::new(MockAuthenticator {
            valid_token: "tok".to_string(),
        });
        let layer = AuthMiddlewareLayer::new(auth).require_auth(false);
        let svc = make_service_with_layer(layer);

        let req = Request::get("/api/data")
            .header("Authorization", "Bearer tok")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"test-user");
    }

    // ---- custom exempt paths ----

    #[tokio::test]
    async fn custom_exempt_paths_override_defaults() {
        let auth: Arc<dyn Authenticator> = Arc::new(RejectAuthenticator);
        let layer = AuthMiddlewareLayer::new(auth)
            .exempt_paths(HashSet::from(["/custom".to_string()]));
        let svc = make_service_with_layer(layer);

        // /custom should be exempt
        let req = Request::get("/custom").body(Body::empty()).unwrap();
        let resp = svc.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // /health should no longer be exempt (defaults replaced)
        let req = Request::get("/health").body(Body::empty()).unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ---- invalid token ----

    #[tokio::test]
    async fn invalid_token_returns_401() {
        let auth: Arc<dyn Authenticator> = Arc::new(MockAuthenticator {
            valid_token: "correct".to_string(),
        });
        let svc = make_service(auth);

        let req = Request::get("/api/data")
            .header("Authorization", "Bearer wrong")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ---- is_exempt unit tests ----

    #[test]
    fn is_exempt_exact_match() {
        let paths = HashSet::from(["/health".to_string()]);
        let prefixes: Vec<String> = vec![];
        assert!(is_path_exempt("/health", &paths, &prefixes));
        assert!(!is_path_exempt("/healthz", &paths, &prefixes));
        assert!(!is_path_exempt("/api", &paths, &prefixes));
    }

    #[test]
    fn is_exempt_prefix_match() {
        let paths = HashSet::new();
        let prefixes = vec!["/public/".to_string(), "/static".to_string()];
        assert!(is_path_exempt("/public/foo", &paths, &prefixes));
        assert!(is_path_exempt("/static", &paths, &prefixes));
        assert!(is_path_exempt("/static/bar", &paths, &prefixes));
        assert!(!is_path_exempt("/api/data", &paths, &prefixes));
    }

    // ---- build_401_response ----

    #[tokio::test]
    async fn build_401_response_has_correct_structure() {
        let resp = build_401_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(resp.headers().get("content-type").unwrap(), "application/json");
        assert_eq!(resp.headers().get("www-authenticate").unwrap(), "Bearer");

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Unauthorized");
        assert_eq!(json["detail"], "Missing or invalid Bearer token");
    }
}
