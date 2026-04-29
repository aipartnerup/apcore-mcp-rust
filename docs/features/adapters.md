# Feature: Adapters

## Module Purpose
Translates between apcore types and MCP protocol types. Covers annotations, errors, schemas, ID normalization, and approval handling.

## Public API Surface

### AnnotationMapper

`AnnotationMapper` is a **unit struct**; methods are associated functions (no `new()` constructor — call them on the type directly).

- `AnnotationMapper::to_mcp_annotations(annotations: Option<&ModuleAnnotations>) -> McpAnnotations`
- `AnnotationMapper::to_description_suffix(annotations: Option<&ModuleAnnotations>) -> String`
- `AnnotationMapper::has_requires_approval(annotations: Option<&ModuleAnnotations>) -> bool`

### ErrorMapper

`ErrorMapper` is a **unit struct**; methods are associated functions.

- `ErrorMapper::to_mcp_error(error: &ModuleError) -> McpErrorResponse`
- `ErrorMapper::to_mcp_error_any<E: std::error::Error + ?Sized>(error: &E) -> McpErrorResponse` — sanitized fallback for non-`ModuleError` inputs
- `ErrorMapper::internal_error_response() -> McpErrorResponse` — `{is_error: true, error_type: "GENERAL_INTERNAL_ERROR", message: "Internal error occurred", details: null}`
- Internal: `INTERNAL_ERROR_CODES` (generic message), `SANITIZED_ERROR_CODES` (hide details), `USER_FIXABLE_ERROR_CODES` (stamps `userFixable=true`)

### SchemaConverter

`SchemaConverter` is a **unit struct**; methods are associated functions.

- `SchemaConverter::convert_input_schema(schema: &Value) -> Result<Value, AdapterError>` (strict mode on)
- `SchemaConverter::convert_output_schema(schema: &Value) -> Result<Value, AdapterError>` (strict mode on)
- `SchemaConverter::convert_input_schema_strict(schema: &Value, strict: bool) -> Result<Value, AdapterError>`
- `SchemaConverter::convert_output_schema_strict(schema: &Value, strict: bool) -> Result<Value, AdapterError>`
- Inlines `$ref` up to depth 32, strips `$defs`, ensures root `type: "object"`. Strict mode injects `additionalProperties: false` on object subschemas (whitelist-walked).

### ModuleIDNormalizer

`ModuleIDNormalizer` is a **unit struct**; methods are associated functions.

- `ModuleIDNormalizer::normalize(module_id: &str) -> Result<String, AdapterError>` (dot → dash, for OpenAI format; validates against `MODULE_ID_PATTERN`)
- `ModuleIDNormalizer::denormalize(tool_name: &str) -> String` (dash → dot, lenient)
- `ModuleIDNormalizer::denormalize_checked(tool_name: &str) -> Result<String, AdapterError>` — bijection-guarded variant; mirrors Python `try_denormalize` / TS `tryDenormalize`. Returns `Err(InvalidModuleId)` when the result fails `MODULE_ID_PATTERN`.

### ElicitationApprovalHandler

- `ElicitationApprovalHandler::new(elicit: Option<ElicitCallback>) -> Self` — `elicit` is the optional constructor-supplied callback used as a fallback when no task-local `ELICIT_CALLBACK` is bound.
- `async request_approval(request: ApprovalRequest) -> ApprovalResult` — reads from `tokio::task_local! ELICIT_CALLBACK` first, then falls back to the constructor field. Catches plugin panics via `futures::FutureExt::catch_unwind` and maps them to a rejected result.
- `async check_approval(approval_id: &str) -> ApprovalResult` — always returns rejected (Phase B `check_approval` is unsupported in this implementation).

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
