//! Shared test helpers for apcore-mcp integration tests.

#![allow(unused)]

/// Create a minimal test registry with sample modules.
pub fn create_test_registry() -> serde_json::Value {
    // TODO: Build a realistic test registry
    serde_json::json!({
        "modules": {}
    })
}

/// Create a mock executor that returns canned responses.
pub fn create_mock_executor() -> serde_json::Value {
    // TODO: Build a mock executor
    serde_json::json!({})
}
