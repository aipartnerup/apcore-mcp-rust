# Feature: Adapters

## Module Purpose
Translates between apcore types and MCP protocol types. Covers annotations, errors, schemas, ID normalization, and approval handling.

## Public API Surface

### AnnotationMapper
- `new() -> AnnotationMapper`
- `to_mcp_annotations(annotations) -> MCP annotations dict`
- `to_description_suffix(annotations) -> String`
- `has_requires_approval(annotations) -> bool`

### ErrorMapper
- `new() -> ErrorMapper`
- `to_mcp_error(error) -> ErrorResponse`
- Internal: INTERNAL_ERROR_CODES (generic message), SANITIZED_ERROR_CODES (hide details)

### SchemaConverter
- `new() -> SchemaConverter`
- `convert_input_schema(descriptor) -> JSON Schema`
- `convert_output_schema(descriptor) -> JSON Schema`
- Inlines $ref, strips $defs, ensures root type: "object"

### ModuleIDNormalizer
- `new() -> ModuleIDNormalizer`
- `normalize(module_id) -> String` (dot -> dash, for OpenAI format)
- `denormalize(tool_name) -> String` (dash -> dot)

### ElicitationApprovalHandler
- `new() -> ElicitationApprovalHandler`
- `async request_approval(request) -> ApprovalResult`
- `async check_approval(approval_id) -> ApprovalResult` (always returns rejected)

## Acceptance Criteria
- [ ] AnnotationMapper maps apcore annotations to MCP hints (readOnlyHint, destructiveHint, etc.)
- [ ] ErrorMapper converts ModuleError to camelCase wire format
- [ ] ErrorMapper sanitizes internal codes (CALL_DEPTH_EXCEEDED, etc.) to generic messages
- [ ] ErrorMapper sanitizes ACL_DENIED to generic "Access denied"
- [ ] ErrorMapper includes AI guidance fields (retryable, aiGuidance, userFixable, suggestion)
- [ ] SchemaConverter inlines $ref up to depth 32
- [ ] SchemaConverter strips $defs after inlining
- [ ] SchemaConverter defaults empty schema to {"type": "object", "properties": {}}
- [ ] ModuleIDNormalizer validates module_id against MODULE_ID_PATTERN
- [ ] ElicitationApprovalHandler reads elicit callback from context
- [ ] ElicitationApprovalHandler maps "accept" -> approved, all else -> rejected
