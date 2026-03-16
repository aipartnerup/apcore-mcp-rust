# Task: define-transport-error

## Goal

Define a `TransportError` enum using `thiserror` to cover all transport failure modes. This provides a unified error type for all transport methods.

## Files Involved

- `src/server/transport.rs` — add `TransportError` enum

## Steps (TDD-first)

1. **Write tests first:**
   - `TransportError::InvalidPort` displays expected message.
   - `TransportError::InvalidHost` displays expected message.
   - `TransportError::Io` wraps a `std::io::Error` and preserves its message.
   - `TransportError` implements `std::error::Error`.
2. **Define `TransportError` enum:**
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum TransportError {
       #[error("invalid host: {0}")]
       InvalidHost(String),

       #[error("port must be between 1 and 65535, got {0}")]
       InvalidPort(u16),

       #[error("I/O error: {0}")]
       Io(#[from] std::io::Error),

       #[error("failed to bind to {host}:{port}: {source}")]
       Bind {
           host: String,
           port: u16,
           source: hyper::Error,
       },

       #[error("server error: {0}")]
       Server(String),
   }
   ```
3. **Implement `From<std::io::Error>` via `#[from]`.**
4. **Run `cargo check`.**

## Acceptance Criteria

- [ ] `TransportError` covers: InvalidHost, InvalidPort, Io, Bind, Server
- [ ] Derives `Debug` and implements `std::error::Error` (via thiserror)
- [ ] `Io` variant has `#[from]` for `std::io::Error`
- [ ] Display messages match expected format
- [ ] All tests pass

## Dependencies

- none

## Estimated Time

30 minutes
