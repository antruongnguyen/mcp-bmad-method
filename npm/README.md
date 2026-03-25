# @bmad-method/mcp-server

npm wrapper for the [BMad Method MCP Server](https://github.com/antruongnguyen/mcp-bmad-method) — a Model Context Protocol server for the BMad Method (Build More Architect Dreams).

## Quick Start

```sh
npx @bmad-method/mcp-server
```

## Install

```sh
npm install -g @bmad-method/mcp-server
```

This downloads a pre-built binary for your platform. If no pre-built binary is available, it falls back to building from source via `cargo install` (requires [Rust](https://rustup.rs/)).

## MCP Client Configuration

### Claude Desktop

```json
{
  "mcpServers": {
    "bmad-method": {
      "command": "npx",
      "args": ["@bmad-method/mcp-server"]
    }
  }
}
```

### Cursor

```json
{
  "mcpServers": {
    "bmad-method": {
      "command": "npx",
      "args": ["@bmad-method/mcp-server"]
    }
  }
}
```

## Alternative: Cargo Install

If you have Rust installed, you can install directly:

```sh
cargo install mcp-bmad-server
```

## Supported Platforms

- macOS (Apple Silicon / arm64)
- macOS (Intel / x86_64)
- Linux (x86_64)
- Linux (arm64)
