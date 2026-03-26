//! Example apcore modules — text_echo, math_calc, greeting.
//!
//! These are self-contained implementations of the `apcore::Module` trait
//! matching the Python examples in `apcore-mcp-python/examples/extensions/`.

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

use apcore::context::Context;
use apcore::errors::{ErrorCode, ModuleError};
use apcore::module::Module;

// ---------------------------------------------------------------------------
// TextEcho — echo text back, optionally uppercase
// ---------------------------------------------------------------------------

pub struct TextEcho;

#[async_trait]
impl Module for TextEcho {
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to echo back"
                },
                "uppercase": {
                    "type": "boolean",
                    "description": "Convert to uppercase",
                    "default": false
                }
            },
            "required": ["text"]
        })
    }

    fn output_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "echoed": { "type": "string", "description": "The echoed text" },
                "length": { "type": "integer", "description": "Character count" }
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

        Ok(json!({
            "echoed": result,
            "length": result.len()
        }))
    }
}

// ---------------------------------------------------------------------------
// MathCalc — basic arithmetic
// ---------------------------------------------------------------------------

pub struct MathCalc;

#[async_trait]
impl Module for MathCalc {
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": { "type": "number", "description": "First operand" },
                "b": { "type": "number", "description": "Second operand" },
                "op": {
                    "type": "string",
                    "description": "Operation: add, sub, mul, div",
                    "default": "add",
                    "enum": ["add", "sub", "mul", "div"]
                }
            },
            "required": ["a", "b"]
        })
    }

    fn output_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "result": { "type": "number", "description": "Calculation result" },
                "expression": { "type": "string", "description": "Human-readable expression" }
            },
            "required": ["result", "expression"]
        })
    }

    fn description(&self) -> &str {
        "Perform basic arithmetic: add, subtract, multiply, or divide"
    }

    async fn execute(&self, inputs: Value, _ctx: &Context<Value>) -> Result<Value, ModuleError> {
        let a = inputs
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'a'"))?;
        let b = inputs
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'b'"))?;
        let op = inputs.get("op").and_then(|v| v.as_str()).unwrap_or("add");

        let (symbol, result) = match op {
            "add" => ("+", a + b),
            "sub" => ("-", a - b),
            "mul" => ("*", a * b),
            "div" => {
                if b == 0.0 {
                    return Err(ModuleError::new(
                        ErrorCode::GeneralInvalidInput,
                        "Division by zero",
                    ));
                }
                ("/", a / b)
            }
            _ => {
                return Err(ModuleError::new(
                    ErrorCode::GeneralInvalidInput,
                    format!("Unknown operation: '{}'. Expected: add, sub, mul, div", op),
                ));
            }
        };

        Ok(json!({
            "result": result,
            "expression": format!("{a} {symbol} {b} = {result}")
        }))
    }
}

// ---------------------------------------------------------------------------
// Greeting — personalized greeting in different styles
// ---------------------------------------------------------------------------

pub struct Greeting;

#[async_trait]
impl Module for Greeting {
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the person to greet"
                },
                "style": {
                    "type": "string",
                    "description": "Greeting style: friendly, formal, pirate",
                    "default": "friendly",
                    "enum": ["friendly", "formal", "pirate"]
                }
            },
            "required": ["name"]
        })
    }

    fn output_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "The greeting message" },
                "timestamp": { "type": "string", "description": "ISO 8601 timestamp" }
            },
            "required": ["message", "timestamp"]
        })
    }

    fn description(&self) -> &str {
        "Generate a personalized greeting in different styles"
    }

    async fn execute(&self, inputs: Value, _ctx: &Context<Value>) -> Result<Value, ModuleError> {
        let name = inputs
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ModuleError::new(ErrorCode::GeneralInvalidInput, "missing 'name'"))?;

        let style = inputs
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("friendly");

        let message = match style {
            "formal" => format!("Good day, {name}. It is a pleasure to make your acquaintance."),
            "pirate" => format!("Ahoy, {name}! Welcome aboard, matey!"),
            _ => format!("Hey {name}! Great to see you!"),
        };

        Ok(json!({
            "message": message,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }
}
