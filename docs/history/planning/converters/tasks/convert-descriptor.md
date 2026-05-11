# Task: convert-descriptor

## Goal

Implement `OpenAIConverter::convert_descriptor` that converts a single apcore `ModuleDescriptor` to an OpenAI-compatible tool definition, composing ID normalization, schema conversion, annotation embedding, and optional strict mode.

## Files Involved

- `src/converters/openai.rs` — Implement `convert_descriptor` method

## Steps (TDD-first)

1. **Write unit tests first**:
   - `test_convert_descriptor_basic` — Minimal descriptor produces `{type: "function", function: {name, description, parameters}}`
   - `test_convert_descriptor_name_normalized` — Dot-separated name `"image.resize"` becomes `"image-resize"`
   - `test_convert_descriptor_schema_converted` — Input schema is processed through `SchemaConverter` (e.g., empty schema becomes `{type: "object", properties: {}}`)
   - `test_convert_descriptor_with_annotations` — When `embed_annotations=true`, description has annotation suffix appended
   - `test_convert_descriptor_without_annotations` — When `embed_annotations=false`, description is unchanged
   - `test_convert_descriptor_with_strict` — When `strict=true`, parameters have strict mode applied and `function.strict: true` is set
   - `test_convert_descriptor_without_strict` — When `strict=false`, no `strict` key in function dict
   - `test_convert_descriptor_strict_transforms_schema` — Verify strict mode actually transforms the parameters (additionalProperties: false, etc.)
   - `test_convert_descriptor_null_annotations_with_embed` — `embed_annotations=true` but annotations are default/empty produces no suffix
   - `test_convert_descriptor_destructive_annotation_warning` — Destructive annotation produces WARNING prefix in description

2. **Define method signature**:
   ```rust
   pub fn convert_descriptor(
       &self,
       name: &str,
       description: &str,
       descriptor: &ModuleDescriptor,
       embed_annotations: bool,
       strict: bool,
   ) -> Result<Value, ConverterError>
   ```

   Note: `name` is the module ID (e.g., `"image.resize"`), `description` comes from the `Module` trait (not on `ModuleDescriptor`). The method accepts them as parameters rather than trying to extract from the descriptor.

3. **Implement the method**:
   - Normalize name: `self.id_normalizer.normalize(name)` (dot -> dash)
   - Convert schema: `SchemaConverter::convert_input_schema(&descriptor.input_schema)`
   - Build description with optional annotation suffix:
     ```rust
     let mut desc = description.to_string();
     if embed_annotations {
         let suffix = AnnotationMapper::to_description_suffix(&annotations_value);
         desc.push_str(&suffix);
     }
     ```
   - Apply strict mode if requested: `self._apply_strict_mode(&parameters)`
   - Build the output JSON:
     ```rust
     let mut function = json!({
         "name": normalized_name,
         "description": desc,
         "parameters": parameters,
     });
     if strict {
         function["strict"] = json!(true);
     }
     Ok(json!({
         "type": "function",
         "function": function,
     }))
     ```

4. **Handle annotation serialization**: `ModuleAnnotations` needs to be converted to a `Value` for `AnnotationMapper::to_description_suffix`. Use `serde_json::to_value(&descriptor.annotations)` or pass the struct directly (depending on the `AnnotationMapper` API once implemented).

5. **Run tests** — ensure all pass. Run `cargo clippy`.

## Acceptance Criteria

- [ ] Output structure matches `{type: "function", function: {name, description, parameters}}`
- [ ] Name is normalized from dots to dashes via `ModuleIDNormalizer`
- [ ] Input schema is converted via `SchemaConverter`
- [ ] Annotations are embedded in description when `embed_annotations=true`
- [ ] Strict mode is applied to parameters when `strict=true`
- [ ] `function.strict: true` is set only when `strict=true`
- [ ] Works with empty/default annotations
- [ ] All tests pass, clippy clean

## Dependencies

- converter-types
- strict-mode

## Estimated Time

2 hours
