//! Plain business logic — NO apcore imports, NO framework dependencies.
//!
//! This file represents an existing project's code that we want to expose
//! as MCP tools without modifying a single line. Mirrors
//! `apcore-mcp-python/examples/binding_demo/myapp.py`.

use std::collections::HashMap;

/// Convert temperature between Celsius, Fahrenheit, and Kelvin.
pub fn convert_temperature(
    value: f64,
    from_unit: &str,
    to_unit: &str,
) -> Result<HashMap<String, String>, String> {
    let celsius = match from_unit {
        "celsius" => value,
        "fahrenheit" => (value - 32.0) * 5.0 / 9.0,
        "kelvin" => value - 273.15,
        _ => return Err(format!("Unknown unit: {from_unit}")),
    };
    let result = match to_unit {
        "celsius" => celsius,
        "fahrenheit" => celsius * 9.0 / 5.0 + 32.0,
        "kelvin" => celsius + 273.15,
        _ => return Err(format!("Unknown unit: {to_unit}")),
    };
    let mut out = HashMap::new();
    out.insert("input".to_string(), format!("{value} {from_unit}"));
    out.insert("output".to_string(), format!("{:.2} {to_unit}", result));
    out.insert("result".to_string(), format!("{:.2}", result));
    Ok(out)
}

/// Count words, characters, and lines in a text string.
pub fn word_count(text: &str) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    out.insert("words".to_string(), text.split_whitespace().count() as u64);
    out.insert("characters".to_string(), text.chars().count() as u64);
    out.insert(
        "lines".to_string(),
        if text.is_empty() {
            0
        } else {
            text.lines().count() as u64
        },
    );
    out
}
