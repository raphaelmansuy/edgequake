# mcp-builder SKILL

## Purpose

Automates the creation, validation, and management of Model Context Protocol (MCP) server projects. Provides standardized project scaffolding, build automation, and integration patterns for MCP-compliant services.

## Features

- Project scaffolding for MCP servers (Rust, Python, TypeScript, etc.)
- Automated build and test workflows
- Validation of MCP compliance (API, schema, endpoints)
- Integration with EdgeQuake and other RAG frameworks
- Documentation and code generation utilities

## Usage

1. Place this SKILL in `.github/skills/mcp-builder/`.
2. Use the provided scripts and templates to scaffold new MCP projects:
   - `scripts/create-mcp-project.sh`
   - `templates/` (starter code, config files)
3. Run validation scripts to ensure MCP compliance.
4. Integrate with CI/CD as needed.

## Example Commands

```bash
# Scaffold a new MCP server in Rust
bash .github/skills/mcp-builder/scripts/create-mcp-project.sh --lang rust --name my-mcp-server

# Validate MCP compliance
python3 .github/skills/mcp-builder/scripts/validate_mcp.py --project my-mcp-server
```

## Directory Structure

- `SKILL.md` (this file)
- `scripts/` (automation scripts)
- `templates/` (starter project templates)
- `README.md` (detailed usage)

## References

- [Model Context Protocol (MCP) Spec](https://github.com/anthropics/model-context-protocol)
- [EdgeQuake RAG Framework](https://github.com/raphaelmansuy/edgequake)
