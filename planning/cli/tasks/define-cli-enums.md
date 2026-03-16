# Task: define-cli-enums

## Goal

Define the `Transport`, `ApprovalMode`, and `LogLevel` enums with clap `ValueEnum` derive for type-safe CLI argument parsing.

## Files Involved

- `src/cli.rs` — add enum definitions

## Steps (TDD-first)

1. **Write tests first:**
   - `Transport::from_str("stdio")` yields `Transport::Stdio`.
   - `Transport::from_str("streamable-http")` yields `Transport::StreamableHttp`.
   - `Transport::from_str("sse")` yields `Transport::Sse`.
   - `ApprovalMode::from_str("elicit")` yields `ApprovalMode::Elicit`.
   - `ApprovalMode::from_str("auto-approve")` yields `ApprovalMode::AutoApprove`.
   - `ApprovalMode::from_str("always-deny")` yields `ApprovalMode::AlwaysDeny`.
   - `ApprovalMode::from_str("off")` yields `ApprovalMode::Off`.
   - `LogLevel::from_str("DEBUG")` yields `LogLevel::Debug`.
   - `LogLevel::from_str("WARNING")` yields `LogLevel::Warning`.
   - Invalid values return errors.
2. **Define `Transport` enum:**
   ```rust
   #[derive(Clone, Debug, PartialEq, ValueEnum)]
   pub enum Transport {
       Stdio,
       #[value(name = "streamable-http")]
       StreamableHttp,
       Sse,
   }
   ```
3. **Define `ApprovalMode` enum:**
   ```rust
   #[derive(Clone, Debug, PartialEq, ValueEnum)]
   pub enum ApprovalMode {
       Elicit,
       #[value(name = "auto-approve")]
       AutoApprove,
       #[value(name = "always-deny")]
       AlwaysDeny,
       Off,
   }
   ```
4. **Define `LogLevel` enum:**
   ```rust
   #[derive(Clone, Debug, PartialEq, ValueEnum)]
   pub enum LogLevel {
       #[value(name = "DEBUG")]
       Debug,
       #[value(name = "INFO")]
       Info,
       #[value(name = "WARNING")]
       Warning,
       #[value(name = "ERROR")]
       Error,
   }
   ```
5. **Implement `LogLevel::to_level_filter()`** mapping to `tracing::level_filters::LevelFilter`:
   - Debug -> LevelFilter::DEBUG
   - Info -> LevelFilter::INFO
   - Warning -> LevelFilter::WARN
   - Error -> LevelFilter::ERROR
6. **Run `cargo check`.**

## Acceptance Criteria

- [ ] All three enums derive `Clone`, `Debug`, `PartialEq`, `ValueEnum`
- [ ] `Transport` variants: Stdio, StreamableHttp ("streamable-http"), Sse
- [ ] `ApprovalMode` variants: Elicit, AutoApprove ("auto-approve"), AlwaysDeny ("always-deny"), Off
- [ ] `LogLevel` variants: Debug ("DEBUG"), Info ("INFO"), Warning ("WARNING"), Error ("ERROR")
- [ ] `LogLevel::to_level_filter()` maps correctly to tracing levels
- [ ] Unit tests pass for all enum parsing

## Dependencies

- none

## Estimated Time

30 minutes
