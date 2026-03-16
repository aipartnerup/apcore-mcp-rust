# Feature: CLI

## Module Purpose
Command-line interface for starting the apcore-mcp server. Parses arguments via clap and delegates to APCoreMCP.

## Public API Surface

### CliArgs (clap derive)
- `--extensions-dir PATH` (required)
- `--transport {stdio, streamable-http, sse}` (default: stdio)
- `--host` (default: 127.0.0.1)
- `--port` (default: 8000, range: 1-65535)
- `--name` (default: "apcore-mcp", max 255)
- `--version` (default: package version)
- `--log-level {DEBUG, INFO, WARNING, ERROR}` (default: INFO)
- `--explorer` (flag)
- `--explorer-prefix` (default: /explorer)
- `--allow-execute` (flag)
- `--jwt-secret` / `--jwt-key-file` / env JWT_SECRET
- `--jwt-algorithm` (default: HS256)
- `--jwt-audience`, `--jwt-issuer`
- `--jwt-require-auth` / `--no-jwt-require-auth` (default: true)
- `--exempt-paths` (comma-separated)
- `--approval {elicit, auto-approve, always-deny, off}` (default: off)

### Functions
- `run() -> Result<()>` — parse args, configure, serve

## Acceptance Criteria
- [ ] All CLI args match Python implementation
- [ ] JWT key resolution: --jwt-key-file > --jwt-secret > JWT_SECRET env
- [ ] Exit codes: 0=normal, 1=invalid args, 2=startup failure
- [ ] Configures tracing subscriber based on --log-level
- [ ] Approval mode selection creates correct handler
- [ ] Extensions directory path is validated
