# Task: implement-cli-args

## Goal

Implement the full `CliArgs` struct with all CLI arguments matching the Python implementation, using clap derive macros.

## Files Involved

- `src/cli.rs` — replace the existing stub `CliArgs` struct

## Steps (TDD-first)

1. **Write tests first:**
   - Parse minimal args: `--extensions-dir /tmp/ext` produces correct defaults for all other fields.
   - Parse full args: all fields populated correctly.
   - Missing `--extensions-dir` produces an error (required arg).
   - `--transport streamable-http` parses correctly.
   - `--approval auto-approve` parses correctly.
   - `--log-level WARNING` parses correctly.
   - `--jwt-require-auth` defaults to `true`.
   - `--no-jwt-require-auth` sets `jwt_require_auth` to `false`.
   - `--exempt-paths "/health,/metrics"` stores the raw string.
   - `--port 0` is rejected (below range 1).
2. **Replace `CliArgs` struct with full definition:**
   ```rust
   #[derive(Parser, Debug)]
   #[command(name = "apcore-mcp", about = "Launch an MCP server that exposes apcore modules as tools.")]
   pub struct CliArgs {
       /// Path to apcore extensions directory.
       #[arg(long)]
       pub extensions_dir: PathBuf,

       /// Transport type.
       #[arg(long, value_enum, default_value_t = Transport::Stdio)]
       pub transport: Transport,

       /// Host address for HTTP-based transports.
       #[arg(long, default_value = "127.0.0.1")]
       pub host: String,

       /// Port for HTTP-based transports (1-65535).
       #[arg(long, default_value_t = 8000, value_parser = clap::value_parser!(u16).range(1..))]
       pub port: u16,

       /// MCP server name (max 255 characters).
       #[arg(long, default_value = "apcore-mcp")]
       pub name: String,

       /// MCP server version (default: package version).
       #[arg(long)]
       pub version: Option<String>,

       /// Logging level.
       #[arg(long, value_enum, default_value_t = LogLevel::Info)]
       pub log_level: LogLevel,

       /// Enable the browser-based Tool Explorer UI.
       #[arg(long, default_value_t = false)]
       pub explorer: bool,

       /// URL prefix for the explorer UI.
       #[arg(long, default_value = "/explorer")]
       pub explorer_prefix: String,

       /// Allow tool execution from the explorer UI.
       #[arg(long, default_value_t = false)]
       pub allow_execute: bool,

       /// JWT secret key for Bearer token authentication.
       #[arg(long)]
       pub jwt_secret: Option<String>,

       /// Path to PEM key file for JWT verification.
       #[arg(long)]
       pub jwt_key_file: Option<PathBuf>,

       /// JWT algorithm.
       #[arg(long, default_value = "HS256")]
       pub jwt_algorithm: String,

       /// Expected JWT audience claim.
       #[arg(long)]
       pub jwt_audience: Option<String>,

       /// Expected JWT issuer claim.
       #[arg(long)]
       pub jwt_issuer: Option<String>,

       /// Require JWT authentication.
       #[arg(long, default_value_t = true, action = ArgAction::Set)]
       pub jwt_require_auth: bool,

       /// Comma-separated paths exempt from auth.
       #[arg(long)]
       pub exempt_paths: Option<String>,

       /// Approval handler mode.
       #[arg(long, value_enum, default_value_t = ApprovalMode::Off)]
       pub approval: ApprovalMode,
   }
   ```
3. **Handle `--jwt-require-auth` / `--no-jwt-require-auth`** using clap's boolean negation pattern. Test both forms.
4. **Remove old fields** (`config`, `validate_inputs`, `include_explorer`, `path_prefix`) that don't match the spec.
5. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `--extensions-dir` is required (PathBuf)
- [ ] `--transport` uses `Transport` enum with default `Stdio`
- [ ] `--host` defaults to `"127.0.0.1"`
- [ ] `--port` defaults to `8000`, validated in range 1-65535
- [ ] `--name` defaults to `"apcore-mcp"`
- [ ] `--version` is optional
- [ ] `--log-level` uses `LogLevel` enum with default `Info`
- [ ] `--explorer` is a boolean flag, default false
- [ ] `--explorer-prefix` defaults to `"/explorer"`
- [ ] `--allow-execute` is a boolean flag, default false
- [ ] `--jwt-secret` is optional
- [ ] `--jwt-key-file` is optional (PathBuf)
- [ ] `--jwt-algorithm` defaults to `"HS256"`
- [ ] `--jwt-audience` and `--jwt-issuer` are optional
- [ ] `--jwt-require-auth` defaults to true, `--no-jwt-require-auth` sets false
- [ ] `--exempt-paths` is optional (raw comma-separated string)
- [ ] `--approval` uses `ApprovalMode` enum with default `Off`
- [ ] All old stub fields removed
- [ ] Unit tests pass for all argument combinations

## Dependencies

- define-cli-enums

## Estimated Time

45 minutes
