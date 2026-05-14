use apcore_mcp::server::router::{ExecutionRouter, OutputFormat};
use async_trait::async_trait;
use serde_json::json;

struct MockExecutor;

#[async_trait]
impl apcore_mcp::server::router::Executor for MockExecutor {
    async fn call_async(
        &self,
        _module_id: &str,
        inputs: &serde_json::Value,
        _context: Option<&serde_json::Value>,
        _version_hint: Option<&str>,
    ) -> Result<serde_json::Value, apcore_mcp::server::router::ExecutorError> {
        Ok(inputs.clone())
    }
}

#[tokio::test]
async fn test_router_output_format_csv() {
    let executor = Box::new(MockExecutor);
    let result = json!([
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ]);

    let router = ExecutionRouter::new(executor, false, None).with_output_format(OutputFormat::Csv);

    let (content, is_error, _) = router.handle_call("test_tool", &result, None).await;

    assert!(!is_error);
    let text = content[0].data.as_str().unwrap();
    assert!(text.contains("id,name"));
    assert!(text.contains("1,Alice"));
    assert!(text.contains("2,Bob"));
}

#[tokio::test]
async fn test_router_output_format_jsonl() {
    let executor = Box::new(MockExecutor);
    let result = json!([
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"}
    ]);

    let router =
        ExecutionRouter::new(executor, false, None).with_output_format(OutputFormat::Jsonl);

    let (content, is_error, _) = router.handle_call("test_tool", &result, None).await;

    assert!(!is_error);
    let text = content[0].data.as_str().unwrap();
    let lines: Vec<&str> = text.trim().split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(lines[0]).unwrap(),
        result[0]
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(lines[1]).unwrap(),
        result[1]
    );
}

#[tokio::test]
async fn test_router_output_format_fallback() {
    let executor = Box::new(MockExecutor);
    let result = json!("not tabular");

    let router = ExecutionRouter::new(executor, false, None).with_output_format(OutputFormat::Csv);

    let (content, is_error, _) = router.handle_call("test_tool", &result, None).await;

    assert!(!is_error);
    let text = content[0].data.as_str().unwrap();
    assert_eq!(text, "\"not tabular\"");
}
