# BMad Method MCP Server

A [Model Context Protocol](https://modelcontextprotocol.io/) server for the [BMad Method](https://docs.bmad-method.org/) (Build More Architect Dreams), implemented in Rust using the [rmcp](https://github.com/modelcontextprotocol/rust-sdk) SDK.

## Prerequisites

- Rust 1.85+ (edition 2024 support)

## Build

```sh
cargo build
```

For an optimized release build:

```sh
cargo build --release
```

## Run

The server communicates over stdio using the MCP protocol:

```sh
cargo run
```

Logs are written to stderr. Set `RUST_LOG` to control log level:

```sh
RUST_LOG=debug cargo run
```

## Lint

```sh
cargo clippy
```

## MCP Client Configuration

### Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "bmad-method": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "/path/to/mcp-bmad-method/Cargo.toml"]
    }
  }
}
```

Or with a pre-built binary:

```json
{
  "mcpServers": {
    "bmad-method": {
      "command": "/path/to/mcp-bmad-method/target/release/mcp-bmad-server"
    }
  }
}
```
