# Contributing to EdgeQuake

Thank you for your interest in EdgeQuake! This document outlines how to contribute to the project.

---

## About EdgeQuake Development

EdgeQuake is developed using a unique **100% automated** approach powered by **edgecode**, a State-of-the-Art (SOTA) coding agent created by **Raphaël MANSUY**.

### Specification-Driven Development

All development in EdgeQuake follows a **Specification-Driven Development** methodology:

1. **Every change starts with a specification** in the `specs/` directory
2. **Specifications are detailed and comprehensive** - they outline objectives, approach, and success criteria
3. **edgecode implements from specifications** - the coding agent reads specs and implements the changes
4. **Iterative OODA Loop** - each iteration produces: Observe → Orient → Decide → Act
5. **No manual coding** - all code changes are generated from specifications

### Example Specification Structure

```
specs/004-documentation-mission/
├── ooda_loop/
│   ├── iteration_01/
│   │   ├── observe.md    # Data gathered, analysis
│   │   ├── orient.md     # Gap analysis, findings
│   │   ├── decide.md     # Action plan, priorities
│   │   └── act.md        # Implementation, changes made
│   ├── iteration_02/
│   │   └── ...
│   └── summary.md        # Cross-iteration insights
```

---

## Current Status: edgecode Is Not Yet Public

**edgecode is being developed and will be released soon.**

Until then, all contributions must go through **Raphaël MANSUY** directly.

---

## How to Contribute

### Bug Reports

Report bugs using GitHub Issues:

1. Go to [EdgeQuake Issues](https://github.com/raphaelmansuy/edgequake/issues)
2. Click "New issue" → "Bug report"
3. Provide:
   - Clear description of the bug
   - Steps to reproduce
   - Expected behavior
   - Actual behavior
   - Environment (OS, Rust version, etc.)

**Example:**

```
Title: PDF extraction fails with multi-column layouts

Description:
When uploading a PDF with 2-column layout, the text appears in wrong order.

Steps to reproduce:
1. Upload test_multicolumn.pdf via UI
2. Check extracted chunks
3. Notice text order is jumbled

Expected: Text follows left-to-right, top-to-bottom reading order
Actual: Text alternates between columns
```

### Feature Requests

Suggest features using GitHub Discussions:

1. Go to [EdgeQuake Discussions](https://github.com/raphaelmansuy/edgequake/discussions)
2. Click "New discussion" → "Ideas"
3. Provide:
   - Clear description of desired feature
   - Use case and rationale
   - Proposed approach (if any)
   - Examples or mockups (if applicable)

**Example:**

```
Title: Support for streaming document uploads

Description:
For large documents (>100MB), streaming upload would improve user experience.

Use Case:
Users with slow connections struggle to upload large PDFs.

Proposed Approach:
- Add chunked upload endpoint: POST /api/v1/documents/stream
- Client sends document in 5MB chunks
- Server reassembles and processes

Benefits:
- Better UX for large files
- Can show progress bar
- Can retry individual chunks on failure
```

### Documentation Improvements

Documentation improvements are welcome! For now:

1. Fork the repository
2. Make documentation changes
3. Submit a pull request
4. Request review from [@raphaelmansuy](https://github.com/raphaelmansuy)

**Documentation changes don't require specifications** - they follow traditional PR workflow.

### Major Contributions

For major features or architectural changes:

1. **Create a specification** in `specs/` following the OODA Loop structure
2. **Describe the change in detail:**
   - Problem statement
   - Proposed solution
   - Why it's needed
   - Impact on existing code
   - Testing strategy
3. **Contact Raphaël MANSUY** for review and implementation
4. edgecode will implement from your specification

---

## Development Workflow

### If You're Developing with edgecode

1. **Write a detailed specification** in `specs/`
2. **Use the OODA Loop structure:**
   - `observe.md` - Analyze current state
   - `orient.md` - Identify gaps and approach
   - `decide.md` - Prioritize and plan
   - `act.md` - Document implementation
3. **Reference the specification file** in your development work
4. **Use the commitment directives:**
   - Commit messages: `OODA-XX: <decision summary>`
   - Include specification files in commits

### If You're Contributing Code Manually

1. **Follow Rust style guidelines:**

   ```bash
   cargo fmt
   cargo clippy
   ```

2. **Write tests:**
   - Unit tests for components
   - Integration tests for workflows
   - E2E tests for user-facing features

3. **Run the full test suite:**

   ```bash
   cargo test
   ```

4. **Check documentation:**
   - Update docs/ if adding new features
   - Keep AGENTS.md up to date if changing workflows

5. **Use conventional commits:**

   ```
   <type>(<scope>): <subject>

   <body>

   <footer>
   ```

   Example:

   ```
   feat(pdf): add table detection enhancement

   Implement enhanced table detection for multi-column layouts.
   Uses vision mode to detect table boundaries before text extraction.

   Fixes #123
   ```

---

## Project Structure

### Source Code

- **Backend**: `edgequake/crates/` - 11 Rust crates
  - `edgequake-core/` - Orchestration
  - `edgequake-llm/` - LLM providers
  - `edgequake-storage/` - Storage adapters
  - `edgequake-api/` - REST API
  - `edgequake-pipeline/` - Document processing
  - `edgequake-query/` - Query engine
  - And 5 more...

- **Frontend**: `edgequake_webui/` - React 19 + TypeScript

### Documentation

- **Main docs**: `docs/` - 44+ files, 10,000+ lines
- **Guidelines**: `AGENTS.md` - Agent workflow
- **Specifications**: `specs/` - OODA Loop iterations

### Tests

- **Backend tests**: `edgequake/crates/*/tests/`
- **Frontend tests**: `edgequake_webui/tests/`
- **E2E tests**: `e2e/`

---

## Code Style

### Rust Code

- **Format**: Use `cargo fmt`
- **Lint**: Use `cargo clippy`
- **Naming**: PascalCase for types, snake_case for functions
- **Comments**: Explain WHY, not WHAT
- **Tests**: Every public API should have tests

Example:

```rust
/// Extracts entities from a text chunk using LLM
///
/// # Arguments
/// * `text` - The text to extract entities from
/// * `entity_types` - Types of entities to extract
///
/// # Returns
/// Vector of extracted entities with confidence scores
pub async fn extract_entities(
    text: &str,
    entity_types: &[EntityType],
) -> Result<Vec<Entity>> {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_entities() {
        // Test implementation
    }
}
```

### TypeScript/React Code

- **Format**: 2-space indentation
- **Lint**: ESLint configuration in place
- **Naming**: camelCase for functions/variables, PascalCase for components
- **Comments**: Explain complex logic
- **Props**: Use TypeScript interfaces

Example:

```typescript
interface DocumentUploadProps {
  onSuccess: (docId: string) => void;
  maxSizeMB?: number;
}

export function DocumentUpload({
  onSuccess,
  maxSizeMB = 100,
}: DocumentUploadProps) {
  // Component implementation
}
```

### Documentation

- **Markdown**: Clear, concise, well-organized
- **Links**: Use relative paths for internal links
- **Code blocks**: Language-specific syntax highlighting
- **Examples**: Provide working examples with expected output
- **ASCII diagrams**: For architecture visualization

---

## Testing

### Backend Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p edgequake-core

# Run specific test
cargo test --test e2e_pipeline

# Run tests with output
cargo test -- --nocapture
```

### Frontend Tests

```bash
# Run all tests
bun test

# Run tests in watch mode
bun test --watch

# Run specific test file
bun test document-upload
```

### Quality Gates

```bash
# Run all quality checks
make test-quality

# Run specific checks
make test-invariants  # Invariant tests
make test-timing      # Test timing (<30s)
make test-count       # Test count (>=2600)
make test-flaky       # Detect flaky tests
```

---

## Making a Pull Request

1. **Fork the repository**

2. **Create a feature branch:**

   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make your changes and commit:**

   ```bash
   git commit -m "feat: add your feature"
   ```

4. **Push to your fork:**

   ```bash
   git push origin feature/your-feature-name
   ```

5. **Create a Pull Request on GitHub:**
   - Clear title and description
   - Link any related issues
   - Include testing instructions

6. **Wait for review from [@raphaelmansuy](https://github.com/raphaelmansuy)**

---

## Contact Information

### For Questions or Collaboration

- **GitHub Issues**: [Bug reports and feature requests](https://github.com/raphaelmansuy/edgequake/issues)
- **GitHub Discussions**: [General questions and ideas](https://github.com/raphaelmansuy/edgequake/discussions)
- **GitHub**: [@raphaelmansuy](https://github.com/raphaelmansuy)
- **LinkedIn**: [raphaelmansuy](https://www.linkedin.com/in/raphaelmansuy)
- **Twitter/X**: [@raphaelmansuy](https://twitter.com/raphaelmansuy)

### For Major Contributions

Email or direct message **Raphaël MANSUY** via:

- LinkedIn: [raphaelmansuy](https://www.linkedin.com/in/raphaelmansuy)
- GitHub: [@raphaelmansuy](https://github.com/raphaelmansuy)

---

## Development Tools

### Required

- **Rust**: 1.78+ ([Install](https://rustup.rs))
- **Node.js**: 18+ or Bun 1.0+ ([Install](https://nodejs.org))
- **Docker**: For PostgreSQL ([Install](https://www.docker.com))

### Recommended

- **VS Code**: With Rust Analyzer
- **Ollama**: For local LLM ([Install](https://ollama.ai))
- **PostgreSQL Client**: `psql` for database debugging

### Make Commands

```bash
# Development
make dev              # Start full stack
make dev-bg           # Start in background
make backend-dev      # Backend only
make frontend-dev     # Frontend only

# Testing
make test             # Run all tests
make lint             # Lint all code
make format           # Format all code

# Maintenance
make clean            # Clean build artifacts
make stop             # Stop all services
```

---

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md). We are committed to providing a welcoming and inclusive environment for all contributors.

---

## License

By contributing to EdgeQuake, you agree that your contributions will be licensed under the Apache License 2.0. See [LICENSE](LICENSE) for details.

---

## Questions?

If you have any questions about contributing, please:

1. Check [AGENTS.md](AGENTS.md) for detailed agent workflow documentation
2. Review [docs/README.md](docs/README.md) for architecture and concepts
3. Ask in [GitHub Discussions](https://github.com/raphaelmansuy/edgequake/discussions)
4. Contact [@raphaelmansuy](https://github.com/raphaelmansuy)

**Thank you for your interest in EdgeQuake!** 🙏

---

## Versioning Workflow (Best Practice)

EdgeQuake uses a unified, automated versioning strategy for both backend (Rust) and frontend (Next.js):

1. **Single Source of Truth:**
   - The root `VERSION` file holds the canonical version.
   - All `Cargo.toml` files and `edgequake_webui/package.json` are updated to match.
2. **Automated Bumping:**
   - Use `make version-bump VERSION=<new_version>` to bump version everywhere.
   - This updates `VERSION`, all `Cargo.toml`, and frontend `package.json`.
3. **Tagging Releases:**
   - Use `make version-tag VERSION=<new_version>` to commit, tag, and push the release.
4. **Changelog:**
   - Update `CHANGELOG.md` after each version bump.
5. **Display:**
   - Version is embedded in backend (via `build.rs`) and shown in the health API and frontend UI.

**Example Release Flow:**

```sh
make version-bump VERSION=0.2.0
# Update CHANGELOG.md
make version-tag VERSION=0.2.0
```

See the Makefile and scripts/bump-version.sh for details.
