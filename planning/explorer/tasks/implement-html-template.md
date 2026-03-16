# Task: implement-html-template

## Goal

Create a minimal, self-contained HTML/JS page for the explorer UI. The page is embedded into the binary via `include_str!` and served at the explorer root. It fetches tool metadata from the `/tools` JSON API and renders a browsable list with schemas and an optional execution form.

## Files Involved

- `src/explorer/templates.rs` — new file with `pub const EXPLORER_HTML: &str` or `pub fn render_html(title, project_name, project_url) -> String`
- `src/explorer/mod.rs` — add `pub mod templates;`

## Steps (TDD-first)

1. **Write a test** that calls `render_html("Test Title", None, None)` and asserts the result contains `<title>Test Title</title>` and the `/tools` fetch URL.

2. **Write a test** that calls `render_html("Title", Some("MyProject"), Some("https://example.com"))` and asserts the footer contains both the project name and URL as a link.

3. **Create `templates.rs`** with a `render_html` function that interpolates title, project_name, and project_url into a template string.

4. **Build the HTML template** with:
   - A `<title>` and `<h1>` using the configured title
   - A `<div id="tools">` container populated by JS
   - JS that fetches `./tools` on load, renders each tool's name, description, and schema
   - If `allow_execute` data attribute is set, render a form per tool with a JSON textarea and submit button that POSTs to `./tools/{name}/call`
   - A footer with optional project name/URL
   - Minimal inline CSS for readability (no external dependencies)

5. **Verify the HTML is valid** (well-formed tags, no external resource loads).

6. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `render_html` produces a complete HTML page
- [ ] Title is configurable and appears in `<title>` and `<h1>`
- [ ] JS fetches `./tools` and renders tool list
- [ ] Tool execution form is conditionally rendered based on a data attribute
- [ ] Project name and URL appear in footer when provided
- [ ] No external resource loads (fully self-contained)
- [ ] Tests verify template interpolation
- [ ] `cargo check` passes

## Dependencies

None.

## Estimated Time

1 hour
