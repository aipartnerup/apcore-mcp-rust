# Feature: OpenAI Converter

## Module Purpose
Converts apcore module registries to OpenAI function-calling tool format. Enables interop with OpenAI-compatible APIs.

## Public API Surface

### OpenAIConverter
- `new() -> OpenAIConverter`
- `convert_registry(registry, embed_annotations, strict, tags, prefix) -> Vec<ToolDef>`
- `convert_descriptor(descriptor, embed_annotations, strict) -> ToolDef`

## Acceptance Criteria
- [ ] Converts apcore descriptors to OpenAI function format: {type: "function", function: {name, description, parameters}}
- [ ] Uses ModuleIDNormalizer to convert dot-notation to dashes
- [ ] Embeds annotations in description when embed_annotations=true
- [ ] Applies strict mode using apcore schema.strict.to_strict_schema() when strict=true
- [ ] Filters by tags when tags are specified
- [ ] Applies prefix to tool names when prefix is specified
