# Changelog (articles)

All notable changes to the EdgeQuake articles are tracked here. See the root CHANGELOG.md for workspace-wide changes.

## [Unreleased]

### Added

- **016-mcp/**: MCP integration announcement post (<1300 chars) following WHY→WHAT→HOW→CLIMAX→CONCLUSION structure
- CHANGELOG.md for articles directory.

### Changed

- Added comprehensive Security section to MCP README covering API Key and OAuth authentication
- Added workspace isolation and best practices documentation
- Added LLM/embedding provider reference tables
- Enhanced API reference with parameter tables for all 17 MCP tools
- Added security warnings in config.ts for production deployments
- Added multi-tenant/workspace auto-discovery warnings in client.ts
- Updated Claude for Desktop integration examples with secure API key configuration

### Added

- Added `document_upload_file` MCP tool for uploading files from file paths
- Support for text files (.txt, .md, .markdown)
- Support for PDF files (.pdf) with automatic routing to PDF upload endpoint
- File type detection and validation with helpful error messages
- File not found error handling with clear user feedback
- Complete documentation with examples in README and API reference

## MCP Version & Git Workflow Best Practices (2026)

### MCP Version Update

- Update version in VERSION, Cargo.toml, or manifest file.
- Commit with: `git commit -m "Bump MCP version to X.Y.Z"`

### Git Branch Management

- Create: `git checkout -b feature/your-branch-name`
- List: `git branch`
- Switch: `git checkout branch-name`

### Git Stage

- Stage all: `git add .`
- Stage file: `git add path/to/file`

### Git Commit

- Commit: `git commit -m "Your concise commit message"`
- Use imperative, focused messages.

### Git Pull Request (PR)

- Push: `git push origin your-branch-name`
- Create PR via GitHub/GitLab UI or CLI:
  - GitHub CLI: `gh pr create --title "Your PR title" --body "Description of changes"`
  - GitLab CLI: `glab mr create --title "Your PR title" --description "Description of changes"`
- Link PR to issues and reviewers.
