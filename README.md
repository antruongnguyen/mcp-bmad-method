# BMad Method MCP Server

A [Model Context Protocol](https://modelcontextprotocol.io/) server for the [BMad Method](https://docs.bmad-method.org/) (Build More Architect Dreams), implemented in Rust using the [rmcp](https://github.com/modelcontextprotocol/rust-sdk) SDK.

## Install

### npx (no install needed)

```sh
npx @bmad-method/mcp-server
```

### npm (global)

```sh
npm install -g @bmad-method/mcp-server
```

### Cargo

```sh
cargo install mcp-bmad-server
```

### From Source

Requires Rust 1.85+ (edition 2024 support).

```sh
cargo build --release
```

## Run

The server communicates over stdio using the MCP protocol (default):

```sh
cargo run
```

### SSE/HTTP Transport

To run as a persistent HTTP service with Server-Sent Events (SSE) transport:

```sh
BMAD_TRANSPORT=sse cargo run
```

This starts an HTTP server on `127.0.0.1:3000` by default. MCP clients connect via `http://127.0.0.1:3000/mcp`.

Configure the host and port with environment variables:

```sh
BMAD_TRANSPORT=sse BMAD_HOST=0.0.0.0 BMAD_PORT=8080 cargo run
```

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `BMAD_TRANSPORT` | `stdio` | Transport mode: `stdio` or `sse` |
| `BMAD_HOST` | `127.0.0.1` | Bind address for SSE mode |
| `BMAD_PORT` | `3000` | Bind port for SSE mode |
| `BMAD_DOCS_URL` | *(unset)* | URL to fetch BMad docs from (enables remote refresh) |
| `BMAD_DOCS_CACHE_PATH` | `~/.cache/bmad-mcp/llms-full.txt` | Path to cache fetched docs on disk |
| `BMAD_ALLOW_REFRESH` | `0` | Set to `1` to enable the `bmad_refresh_docs` tool |
| `RUST_LOG` | `info` | Log level filter (e.g. `debug`, `warn`) |

Logs are written to stderr. Stdout is reserved for MCP protocol messages.

## Tools

The server exposes 13 MCP tools:

### Workflow Navigation

| Tool | Description |
|---|---|
| `bmad_get_workflow` | Get metadata for a workflow by skill id (phase, agent, outputs, prerequisites, next steps) |
| `bmad_get_next_steps` | Get recommended next-step workflows after completing a phase |
| `bmad_get_track_workflows` | List all workflows for a planning track (Quick Flow, BMad Method, Enterprise) |

### Project Guidance

| Tool | Description |
|---|---|
| `bmad_next_step` | Recommend the next workflow based on current project state (free-text or JSON input) |
| `bmad_help` | Answer questions about the BMad Method — phases, agents, workflows, tracks, and tools |
| `bmad_check_readiness` | Validate whether a project is ready to enter the Implementation phase |
| `bmad_sprint_guide` | Guide through the Implementation build cycle: story creation, implementation, review, retrospective |

### Project & Agent Discovery

| Tool | Description |
|---|---|
| `bmad_list_agents` | List all BMad agents, optionally filtered by phase |
| `bmad_agent_info` | Get detailed info about a specific agent by skill id |
| `bmad_project_state` | Scan a project directory to detect existing BMad artifacts and infer current phase |
| `bmad_scaffold` | Generate starter BMad project files (`_bmad/` directory and planning stubs) for a given track |

### Server Management

| Tool | Description |
|---|---|
| `bmad_refresh_docs` | Refresh the documentation cache from a remote URL (requires `BMAD_ALLOW_REFRESH=1`) |
| `bmad_index_status` | Return diagnostic info about the index: doc source, refresh time, workflow/agent counts |

### JSON Output Mode

All tools accept an optional `output_format` parameter. Set it to `"json"` to receive structured JSON instead of the default markdown output. This is useful for programmatic consumption by other tools or scripts.

## Docker

Build and run via Docker:

```sh
docker build -t mcp-bmad-server .
docker run -i mcp-bmad-server
```

For SSE mode:

```sh
docker run -e BMAD_TRANSPORT=sse -e BMAD_HOST=0.0.0.0 -p 3000:3000 mcp-bmad-server
```

## MCP Client Configuration

Ready-to-use config snippets are in the `example/` directory.

### Claude Desktop

Add to your `claude_desktop_config.json` (see `example/claude_desktop_config.json`):

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

Or if you installed the binary directly:

```json
{
  "mcpServers": {
    "bmad-method": {
      "command": "mcp-bmad-server"
    }
  }
}
```

For SSE mode, connect your client to `http://127.0.0.1:3000/mcp`.

### Cursor

Add to your MCP settings (see `example/cursor_mcp.json`):

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

## Lint

```sh
cargo clippy
```

## Test

```sh
cargo test
```

## License

MIT OR Apache-2.0
