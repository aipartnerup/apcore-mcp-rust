//! Integration tests for the adapters module.

mod common;

use apcore_mcp::{AnnotationMapper, ErrorMapper, SchemaConverter};
use serde_json::json;

// ---- AnnotationMapper integration tests ------------------------------------

#[test]
fn annotation_mapper_mcp_extras_stripped_and_colon_format() {
    // [D11-021] mcp_ prefix stripped, colon separator, outside [Annotations:]
    use apcore::module::ModuleAnnotations;
    let mut extra = std::collections::HashMap::new();
    extra.insert("mcp_cost_usd".to_string(), json!("0.05"));
    extra.insert("mcp_model".to_string(), json!("gpt-4"));
    let ann = ModuleAnnotations {
        extra,
        ..Default::default()
    };
    let suffix = AnnotationMapper::to_description_suffix(Some(&ann));
    assert!(
        suffix.contains("cost_usd: 0.05"),
        "must use stripped key with colon: {suffix:?}"
    );
    assert!(
        suffix.contains("model: gpt-4"),
        "must use stripped key with colon: {suffix:?}"
    );
    assert!(
        !suffix.contains("mcp_cost_usd"),
        "mcp_ prefix must be stripped: {suffix:?}"
    );
}

#[test]
fn annotation_mapper_destructive_produces_warning() {
    use apcore::module::ModuleAnnotations;
    let ann = ModuleAnnotations {
        destructive: true,
        ..Default::default()
    };
    let suffix = AnnotationMapper::to_description_suffix(Some(&ann));
    assert!(suffix.contains("DESTRUCTIVE"));
    assert!(suffix.contains("WARNING"));
}

// ---- ErrorMapper integration tests -----------------------------------------

#[test]
fn error_mapper_internal_errors_sanitized() {
    use apcore::errors::{ErrorCode, ModuleError};
    let err = ModuleError::new(ErrorCode::CallDepthExceeded, "depth exceeded");
    let resp = ErrorMapper::to_mcp_error(&err);
    assert!(resp.is_error);
    assert_eq!(resp.message, "Internal error occurred");
    assert!(
        resp.details.is_none(),
        "internal errors must not leak details"
    );
}

#[test]
fn error_mapper_any_with_io_error_returns_internal_error() {
    // [D10-009] Arbitrary error types fall back to INTERNAL_ERROR.
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let resp = ErrorMapper::to_mcp_error_any(&io_err);
    assert!(resp.is_error);
    assert_eq!(resp.error_type, "INTERNAL_ERROR");
}

// ---- SchemaConverter integration tests -------------------------------------

#[test]
fn schema_converter_roundtrip_simple_object() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
        },
        "required": ["name"]
    });
    let result = SchemaConverter::convert_input_schema(&schema).unwrap();
    assert_eq!(result["type"], "object");
    assert_eq!(result["properties"]["name"]["type"], "string");
    assert_eq!(result["properties"]["age"]["type"], "integer");
    assert_eq!(result["required"], json!(["name"]));
    // Strict mode adds additionalProperties: false
    assert_eq!(result["additionalProperties"], json!(false));
}

#[test]
fn schema_converter_defs_inlined_and_stripped() {
    let schema = json!({
        "type": "object",
        "properties": {
            "config": {"$ref": "#/$defs/Config"}
        },
        "$defs": {
            "Config": {
                "type": "object",
                "properties": {
                    "enabled": {"type": "boolean"}
                }
            }
        }
    });
    let result = SchemaConverter::convert_input_schema(&schema).unwrap();
    assert!(
        result.get("$defs").is_none(),
        "$defs must be stripped after inlining"
    );
    assert_eq!(
        result["properties"]["config"]["properties"]["enabled"]["type"],
        "boolean"
    );
}
