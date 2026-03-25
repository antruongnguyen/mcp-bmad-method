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

## Release Binary

Build an optimized release binary:

```sh
cargo build --release
```

The binary is at `target/release/mcp-bmad-server`. It communicates over stdio so it can be used directly with any MCP client.

## Docker

Build and run via Docker:

```sh
docker build -t mcp-bmad-server .
docker run -i mcp-bmad-server
```

## MCP Client Configuration

Ready-to-use config snippets are in the `example/` directory.

### Claude Desktop

Add to your `claude_desktop_config.json` (see `example/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "bmad-method": {
      "command": "/path/to/mcp-bmad-server"
    }
  }
}
```

### Cursor

Add to your MCP settings (see `example/cursor_mcp.json`):

```json
{
  "mcpServers": {
    "bmad-method": {
      "command": "/path/to/mcp-bmad-server"
    }
  }
}
```
