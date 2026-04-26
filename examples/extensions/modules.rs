//! Discoverable modules — each fn here mirrors a single Python file in
//! `apcore-mcp-python/examples/extensions/`. `register_all` enumerates
//! them via a single sweep, mimicking what apcore's `extensions_dir`
//! filesystem-discovery would produce.
//!
//! Production code with many modules would use a Discoverer impl
//! (`apcore::registry::Discoverer`) registered on the Registry, but the
//! enumeration pattern shown here keeps the example dependency-light
//! and requires no filesystem traversal.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

use apcore::context::Context;
use apcore::errors::{ErrorCode, ModuleError};
use apcore::module::{Module, ModuleAnnotations};
use apcore::registry::registry::{ModuleDescriptor, Registry};

// ---------------------------------------------------------------------------
// text.echo
// ---------------------------------------------------------------------------

pub struct TextEcho;

#[async_trait]
impl Module for TextEcho {
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"},
                "uppercase": {"type": "boolean", "default": false}
            },
            "required": ["text"]
        })
    }
    fn output_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "echoed": {"type": "string"},
                "length": {"type": "integer"}
            },
            "required": ["echoed", "length"]
        })
    }
    fn description(&self) -> &str {
        "Echo input text back, optionally converting to uppercase"
    }
    async fn execute(&self, inputs: Value, _ctx: &Context<Value>) -> Result<Value, ModuleError> {
        let text = inputs
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'text'"))?;
        let uppercase = inputs
            .get("uppercase")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let result = if uppercase {
            text.to_uppercase()
        } else {
            text.to_string()
        };
        Ok(json!({"echoed": result, "length": result.len()}))
    }
}

// ---------------------------------------------------------------------------
// math.calc
// ---------------------------------------------------------------------------

pub struct MathCalc;

#[async_trait]
impl Module for MathCalc {
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "op": {"type": "string", "enum": ["add", "sub", "mul", "div"]},
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["op", "a", "b"]
        })
    }
    fn output_schema(&self) -> Value {
        json!({"type": "object", "properties": {"result": {"type": "number"}}, "required": ["result"]})
    }
    fn description(&self) -> &str {
        "Basic arithmetic — add, sub, mul, div"
    }
    async fn execute(&self, inputs: Value, _ctx: &Context<Value>) -> Result<Value, ModuleError> {
        let op = inputs
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'op'"))?;
        let a = inputs
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'a'"))?;
        let b = inputs
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'b'"))?;
        let result = match op {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => {
                if b == 0.0 {
                    return Err(ModuleError::new(
                        ErrorCode::GeneralInvalidInput,
                        "division by zero",
                    ));
                }
                a / b
            }
            _ => {
                return Err(ModuleError::new(
                    ErrorCode::GeneralInvalidInput,
                    format!("unknown op: {op}"),
                ))
            }
        };
        Ok(json!({"result": result}))
    }
}

// ---------------------------------------------------------------------------
// greeting
// ---------------------------------------------------------------------------

pub struct Greeting;

#[async_trait]
impl Module for Greeting {
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "style": {"type": "string", "enum": ["casual", "formal"], "default": "casual"}
            },
            "required": ["name"]
        })
    }
    fn output_schema(&self) -> Value {
        json!({"type": "object", "properties": {"message": {"type": "string"}}, "required": ["message"]})
    }
    fn description(&self) -> &str {
        "Personalized greeting in different styles"
    }
    async fn execute(&self, inputs: Value, _ctx: &Context<Value>) -> Result<Value, ModuleError> {
        let name = inputs
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'name'"))?;
        let style = inputs
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("casual");
        let message = match style {
            "formal" => format!("Good day, {name}."),
            _ => format!("Hi {name}!"),
        };
        Ok(json!({"message": message}))
    }
}

// ---------------------------------------------------------------------------
// register_all — single sweep, equivalent to filesystem discovery output
// ---------------------------------------------------------------------------

/// One module's metadata + impl, batched together for `register_all`.
type ModuleEntry = (
    &'static str,
    &'static str,
    Vec<&'static str>,
    Box<dyn Module>,
);

pub fn register_all(registry: &Registry) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<ModuleEntry> = vec![
        (
            "text.echo",
            "Echo input text back",
            vec!["text", "utility"],
            Box::new(TextEcho),
        ),
        (
            "math.calc",
            "Basic arithmetic",
            vec!["math", "utility"],
            Box::new(MathCalc),
        ),
        (
            "greeting",
            "Personalized greeting",
            vec!["text", "social"],
            Box::new(Greeting),
        ),
    ];

    for (module_id, description, tags, module) in entries {
        let descriptor = ModuleDescriptor {
            module_id: module_id.into(),
            name: None,
            description: description.into(),
            documentation: None,
            input_schema: module.input_schema(),
            output_schema: module.output_schema(),
            version: "1.0.0".into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            annotations: Some(ModuleAnnotations {
                readonly: true,
                idempotent: true,
                open_world: false,
                ..Default::default()
            }),
            examples: vec![],
            metadata: HashMap::new(),
            display: None,
            sunset_date: None,
            dependencies: vec![],
            enabled: true,
        };
        registry.register(module_id, module, descriptor)?;
    }

    Ok(())
}
