# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] — 2026-03-25

### Added
- `bmad_scaffold` tool for generating starter project files with track-appropriate boilerplate (M13)
- JSON output mode (`output_format: "json"`) on all tools for programmatic consumption (M15)
- SSE/HTTP transport option alongside stdio (`BMAD_TRANSPORT=sse`) (M12)
- `bmad_index_status` tool for index diagnostics (M14)
- Error recovery with retry logic and doc validation on `bmad_refresh_docs` (M14)
- Cross-platform release CI with prebuilt binaries (M18)
- npm wrapper package `@bmad-method/mcp-server` for npx/global install (M16)

### Changed
- Bumped version from 0.9.0 to 1.0.0

## [0.9.0] — 2026-03-25

### Added
- `bmad_project_state` tool for combined project state snapshot with inferred workflows and readiness (M10)
- Integration test suite covering all MCP tool handlers (M9)
- `bmad_sprint_guide` tool for story-by-story build cycle guidance (M7)
- Lazy-init index with `Arc<RwLock>` for concurrent access (M8)
- `bmad_refresh_docs` tool for HTTP-based remote documentation refresh (M8)
- `bmad_check_readiness` tool for implementation readiness validation (M6)
- `bmad_list_agents` and `bmad_agent_info` tools for agent discovery and detail lookup (M5)
- CI pipeline, Dockerfile, and release packaging (M4)
- `bmad_get_workflow`, `bmad_get_next_steps`, and `bmad_get_track_workflows` tools for workflow navigation (M3)
- `bmad_next_step` and `bmad_help` tools for guided recommendations and Q&A (M2)
- BMad Method content loader and in-memory index from `llms-full.txt` (M1)
- Rust workspace with MCP server skeleton using `rmcp` SDK (M0)
- Example configs for Claude Desktop and Cursor MCP integration

[Unreleased]: https://github.com/antruongnguyen/mcp-bmad-method/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/antruongnguyen/mcp-bmad-method/compare/v0.9.0...v1.0.0
[0.9.0]: https://github.com/antruongnguyen/mcp-bmad-method/releases/tag/v0.9.0
