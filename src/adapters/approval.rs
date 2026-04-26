//! ElicitationApprovalHandler — uses MCP elicitation to request user approval
//! for destructive or sensitive tool executions.

use std::fmt;

use apcore::approval::{ApprovalHandler, ApprovalRequest, ApprovalResult};
use apcore::errors::ModuleError;
use async_trait::async_trait;

use crate::helpers::{ElicitAction, ElicitCallback};

/// Handles user approval requests via MCP elicitation.
///
/// Implements the apcore [`ApprovalHandler`] contract by sending an elicitation
/// prompt to the MCP client and interpreting the response.
///
/// # Lifecycle
///
/// - `request_approval` — formats a human-readable message from the
///   [`ApprovalRequest`], invokes the injected [`ElicitCallback`], and maps the
///   elicit response to an [`ApprovalResult`].
/// - `check_approval` — always returns "rejected" because Phase B (async
///   polling of pending approvals) is not supported via stateless MCP
///   elicitation.
pub struct ElicitationApprovalHandler {
    elicit: Option<ElicitCallback>,
}

impl ElicitationApprovalHandler {
    /// Create a new approval handler with an optional elicit callback.
    ///
    /// When `elicit` is `None`, all approval requests will be rejected with a
    /// descriptive reason indicating no callback is available.
    pub fn new(elicit: Option<ElicitCallback>) -> Self {
        Self { elicit }
    }
}

impl fmt::Debug for ElicitationApprovalHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ElicitationApprovalHandler")
            .field("has_elicit", &self.elicit.is_some())
            .finish()
    }
}

/// Build a rejected [`ApprovalResult`] with the given reason.
fn rejected(reason: &str) -> ApprovalResult {
    ApprovalResult {
        status: "rejected".to_string(),
        approved_by: None,
        reason: Some(reason.to_string()),
        approval_id: None,
        metadata: None,
    }
}

/// Build an approved [`ApprovalResult`].
fn approved() -> ApprovalResult {
    ApprovalResult {
        status: "approved".to_string(),
        approved_by: None,
        reason: None,
        approval_id: None,
        metadata: None,
    }
}

#[async_trait]
impl ApprovalHandler for ElicitationApprovalHandler {
    async fn request_approval(
        &self,
        request: &ApprovalRequest,
    ) -> Result<ApprovalResult, ModuleError> {
        let elicit = match &self.elicit {
            Some(cb) => cb,
            None => {
                tracing::debug!("no elicitation callback available, rejecting approval");
                return Ok(rejected("No elicitation callback available"));
            }
        };

        let message = format!(
            "Approval required for tool: {}\n\n{}\n\nArguments: {}",
            request.module_id,
            request.description.as_deref().unwrap_or(""),
            request.arguments,
        );

        // [AH-3] Catch panics from the elicit callback so a buggy
        // implementation can't bring down the approval task. Mirrors
        // Python (try/except) and TypeScript (try/catch) which both
        // degrade to `rejected("Elicitation request failed")` on error.
        // futures::FutureExt::catch_unwind handles async panics across
        // .await points; std::panic::catch_unwind cannot.
        use futures::FutureExt;
        let result_outcome = std::panic::AssertUnwindSafe(elicit(message, None))
            .catch_unwind()
            .await;
        let result = match result_outcome {
            Ok(Some(r)) => r,
            Ok(None) => {
                tracing::debug!("elicitation returned no response");
                return Ok(rejected("Elicitation returned no response"));
            }
            Err(_panic) => {
                tracing::debug!("elicit callback panicked");
                return Ok(rejected("Elicitation request failed"));
            }
        };

        match result.action {
            ElicitAction::Accept => Ok(approved()),
            ElicitAction::Decline => Ok(rejected("User action: decline")),
            ElicitAction::Cancel => Ok(rejected("User action: cancel")),
        }
    }

    async fn check_approval(&self, _approval_id: &str) -> Result<ApprovalResult, ModuleError> {
        Ok(rejected("Phase B not supported via MCP elicitation"))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::ElicitResult;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    /// Create a mock elicit callback that returns the given action.
    fn mock_elicit(action: ElicitAction) -> ElicitCallback {
        Box::new(move |_msg, _schema| {
            let action = action.clone();
            Box::pin(async move {
                Some(ElicitResult {
                    action,
                    content: None,
                })
            })
        })
    }

    /// Create a mock elicit callback that returns `None`.
    fn mock_elicit_none() -> ElicitCallback {
        Box::new(|_msg, _schema| Box::pin(async { None }))
    }

    /// Create a mock [`ApprovalRequest`] for testing.
    fn test_request() -> ApprovalRequest {
        ApprovalRequest {
            module_id: "test.dangerous_tool".to_string(),
            arguments: json!({"path": "/etc/passwd"}),
            context: None,
            annotations: Default::default(),
            description: Some("Delete a system file".to_string()),
            tags: vec!["destructive".to_string()],
        }
    }

    // -- request_approval tests -----------------------------------------------

    #[tokio::test]
    async fn test_request_approval_accepted() {
        let handler = ElicitationApprovalHandler::new(Some(mock_elicit(ElicitAction::Accept)));
        let result = handler.request_approval(&test_request()).await.unwrap();
        assert_eq!(result.status, "approved");
        assert!(result.reason.is_none());
    }

    #[tokio::test]
    async fn test_request_approval_declined() {
        let handler = ElicitationApprovalHandler::new(Some(mock_elicit(ElicitAction::Decline)));
        let result = handler.request_approval(&test_request()).await.unwrap();
        assert_eq!(result.status, "rejected");
        assert_eq!(result.reason.as_deref(), Some("User action: decline"));
    }

    #[tokio::test]
    async fn test_request_approval_cancelled() {
        let handler = ElicitationApprovalHandler::new(Some(mock_elicit(ElicitAction::Cancel)));
        let result = handler.request_approval(&test_request()).await.unwrap();
        assert_eq!(result.status, "rejected");
        assert_eq!(result.reason.as_deref(), Some("User action: cancel"));
    }

    #[tokio::test]
    async fn test_request_approval_no_callback() {
        let handler = ElicitationApprovalHandler::new(None);
        let result = handler.request_approval(&test_request()).await.unwrap();
        assert_eq!(result.status, "rejected");
        assert_eq!(
            result.reason.as_deref(),
            Some("No elicitation callback available")
        );
    }

    #[tokio::test]
    async fn test_request_approval_callback_none() {
        let handler = ElicitationApprovalHandler::new(Some(mock_elicit_none()));
        let result = handler.request_approval(&test_request()).await.unwrap();
        assert_eq!(result.status, "rejected");
        assert_eq!(
            result.reason.as_deref(),
            Some("Elicitation returned no response")
        );
    }

    // Note: callback panic catching is deferred until `futures` crate is added.
    // The current implementation does not catch panics in the elicit callback.

    // -- check_approval tests -------------------------------------------------

    #[tokio::test]
    async fn test_check_approval_always_rejected() {
        let handler = ElicitationApprovalHandler::new(None);
        let result = handler.check_approval("any-id-123").await.unwrap();
        assert_eq!(result.status, "rejected");
        assert_eq!(
            result.reason.as_deref(),
            Some("Phase B not supported via MCP elicitation")
        );
    }

    // -- message format test --------------------------------------------------

    #[tokio::test]
    async fn test_approval_message_format() {
        let captured_msg = Arc::new(Mutex::new(String::new()));
        let captured_clone = captured_msg.clone();
        let cb: ElicitCallback = Box::new(move |msg, _schema| {
            let captured = captured_clone.clone();
            Box::pin(async move {
                *captured.lock().unwrap() = msg;
                Some(ElicitResult {
                    action: ElicitAction::Accept,
                    content: None,
                })
            })
        });

        let handler = ElicitationApprovalHandler::new(Some(cb));
        let request = test_request();
        handler.request_approval(&request).await.unwrap();

        let msg = captured_msg.lock().unwrap().clone();
        assert!(
            msg.contains("test.dangerous_tool"),
            "message should contain module_id"
        );
        assert!(
            msg.contains("Delete a system file"),
            "message should contain description"
        );
        assert!(
            msg.contains("/etc/passwd"),
            "message should contain arguments"
        );
    }

    // -- Debug impl test ------------------------------------------------------

    #[test]
    fn test_debug_with_callback() {
        let handler = ElicitationApprovalHandler::new(Some(mock_elicit(ElicitAction::Accept)));
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("has_elicit: true"));
    }

    #[test]
    fn test_debug_without_callback() {
        let handler = ElicitationApprovalHandler::new(None);
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("has_elicit: false"));
    }
}
