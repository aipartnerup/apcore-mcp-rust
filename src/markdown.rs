//! Markdown rendering for apcore modules via apcore-toolkit.
//!
//! LLMs read MCP/OpenAI tool `description` strings as their primary
//! signal for tool selection — the richer the description, the better
//! the agent picks the right tool. apcore-toolkit's
//! `format_module(style = Markdown)` emits a canonical, cross-SDK
//! byte-equivalent rendering with title, description, parameters,
//! returns, behavior table, tags, and examples.
//!
//! This module bridges apcore's `ModuleDescriptor` (the runtime type
//! flowing through apcore-mcp) to apcore-toolkit's `ScannedModule`
//! (the input format `format_module` expects), then delegates.

use std::collections::HashMap;

use apcore::registry::ModuleDescriptor;
use apcore_toolkit::{format_module, FormatOutput, ModuleStyle, ScannedModule};

/// Adapt an apcore [`ModuleDescriptor`] to a toolkit [`ScannedModule`].
///
/// The two types are near-supersets of each other — overlapping fields
/// are copied verbatim and toolkit-only fields (`target`,
/// `documentation`, `suggested_alias`, `warnings`) get sensible
/// defaults so `format_module` produces identical output regardless of
/// which type the caller starts from.
pub fn descriptor_to_scanned_module(descriptor: &ModuleDescriptor) -> ScannedModule {
    ScannedModule {
        module_id: descriptor.module_id.clone(),
        description: descriptor.description.clone(),
        input_schema: descriptor.input_schema.clone(),
        output_schema: descriptor.output_schema.clone(),
        tags: descriptor.tags.clone(),
        // ModuleDescriptor doesn't carry a callable target string; emit
        // an empty placeholder. format_module(markdown) doesn't render
        // `target` so this has no observable effect on the output.
        target: String::new(),
        version: descriptor.version.clone(),
        annotations: descriptor.annotations.clone(),
        documentation: descriptor.documentation.clone(),
        suggested_alias: None,
        examples: descriptor.examples.clone(),
        // Convert metadata: ModuleDescriptor uses HashMap<String, Value>,
        // ScannedModule does too — same shape, direct clone.
        metadata: descriptor
            .metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<_, _>>(),
        display: descriptor.display.clone(),
        warnings: vec![],
    }
}

/// Render a [`ModuleDescriptor`] as canonical apcore-toolkit Markdown.
///
/// Returns the Markdown body produced by
/// `format_module(scanned, ModuleStyle::Markdown, display)` — title,
/// description, parameters list, returns list, behavior table (toolkit
/// 0.6.0 emits only fields differing from defaults), tags, and
/// examples. Falls back to the descriptor's plain `description` when
/// the toolkit returns a non-text variant (defensive — should never
/// happen for `Markdown` style).
pub fn render_module_markdown(descriptor: &ModuleDescriptor, display: bool) -> String {
    let scanned = descriptor_to_scanned_module(descriptor);
    match format_module(&scanned, ModuleStyle::Markdown, display) {
        FormatOutput::Text(text) => text,
        _ => descriptor.description.clone(),
    }
}
