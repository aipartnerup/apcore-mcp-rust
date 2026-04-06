//! Authenticator trait — defines the authentication contract.
//!
//! Implementations inspect request headers and return an identity on success.
//! The [`Identity`] type is re-exported from the core `apcore` crate to ensure
//! a single canonical identity model across the stack.

use std::collections::HashMap;

use async_trait::async_trait;

pub use apcore::Identity;

/// Trait for authenticating incoming MCP requests.
///
/// Implementors inspect HTTP headers (or equivalent metadata) and return
/// an [`Identity`] if authentication succeeds, or `None` if it fails.
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Authenticate a request based on its headers.
    ///
    /// Returns `Some(Identity)` on success, `None` on failure.
    async fn authenticate(&self, headers: &HashMap<String, String>) -> Option<Identity>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

    #[test]
    fn identity_is_apcore_identity() {
        assert_eq!(
            TypeId::of::<Identity>(),
            TypeId::of::<apcore::Identity>(),
            "protocol::Identity must be the same type as apcore::Identity"
        );
    }

    #[test]
    fn identity_has_expected_fields() {
        let identity = Identity::new(
            "user-123".to_string(),
            "human".to_string(),
            vec!["admin".to_string()],
            Default::default(),
        );
        assert_eq!(identity.id(), "user-123");
        assert_eq!(identity.identity_type(), "human");
        assert_eq!(identity.roles(), vec!["admin"]);
        assert!(identity.attrs().is_empty());
    }

    #[test]
    fn identity_debug_and_clone() {
        let identity = Identity::new(
            "agent-1".to_string(),
            "service".to_string(),
            vec![],
            Default::default(),
        );
        let cloned = identity.clone();
        assert_eq!(format!("{:?}", identity), format!("{:?}", cloned));
    }

    #[tokio::test]
    async fn authenticator_trait_is_object_safe() {
        // Verify that Authenticator can be used as a trait object.
        struct NoOpAuth;

        #[async_trait]
        impl Authenticator for NoOpAuth {
            async fn authenticate(&self, _headers: &HashMap<String, String>) -> Option<Identity> {
                None
            }
        }

        let auth: Box<dyn Authenticator> = Box::new(NoOpAuth);
        let result = auth.authenticate(&HashMap::new()).await;
        assert!(result.is_none());
    }
}
