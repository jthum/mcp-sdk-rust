# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-14

### Added
- **Normalization**: Added `CallToolResult::as_text()` helper to simplify converting mixed MCP content (text, images, resources) into a single string for LLM consumption.
- **Error Handling**: Implemented `std::fmt::Display` and `std::error::Error` for `JsonRpcError`, enabling better integration with `anyhow` and other error handling libraries.

### Changed
- Standardized internal `ToolDefinition` accessors.

## [0.1.0] - 2026-02-13

### Added
- Initial release.
- Stdio transport for subprocess-based MCP servers.
- JSON-RPC 2.0 client implementation.
- Basic support for `initialize`, `tools/list`, and `tools/call`.
