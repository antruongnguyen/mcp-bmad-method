# Project Conventions

## Language & Toolchain
- Rust, edition 2024, resolver 3
- Async runtime: tokio (single-threaded `current_thread`)
- MCP SDK: rmcp with `server` + `transport-io` features

## Workspace Layout
- `Cargo.toml` — workspace root
- `mcp-bmad-server/` — main server crate

## Code Style
- Follow `cargo clippy` — all code must pass with no warnings
- Use `tracing` for logging (never `println!` or `eprintln!`)
- Logs go to stderr; stdout is reserved for MCP protocol messages
- Error handling: use `anyhow::Result` in main, `rmcp::ErrorData` in tool handlers

## Commit Messages
- Co-author line required: `Co-Authored-By: Paperclip <noreply@paperclip.ing>`
