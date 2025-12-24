# Reverse Documentation Skill for Rust

This skill enables you to analyze Rust codebases and generate comprehensive documentation by understanding code structure, patterns, and relationships.

## Purpose

Automatically generate documentation for Rust projects by analyzing:

- Module structure and organization
- Type definitions (structs, enums, traits)
- Function signatures and implementations
- Error handling patterns
- Async/await patterns
- Test coverage
- Dependencies and feature flags

## Usage

To use this skill, provide one or more Rust files or directories and specify the documentation format you want.

### Example Commands

```
Generate comprehensive documentation for the edgequake-core crate
```

```
Document the storage trait implementations in edgequake-storage
```

```
Create API documentation for all public interfaces in the edgequake workspace
```

## Skill Workflow

When you invoke this skill, the agent will:

1. **Analyze Code Structure**

   - Parse Rust source files
   - Identify modules, types, traits, and functions
   - Extract public APIs and internal implementation details

2. **Extract Metadata**

   - Read Cargo.toml for dependencies and features
   - Identify crate relationships
   - Extract version information and metadata

3. **Understand Patterns**

   - Identify design patterns (Builder, Factory, Strategy, etc.)
   - Recognize async patterns and tokio usage
   - Understand error handling approaches (Result<T>, thiserror, anyhow)
   - Detect trait implementations and polymorphism

4. **Generate Documentation**
   - Create module-level documentation
   - Generate function and type documentation
   - Document public APIs with examples
   - Create architecture diagrams (optional)
   - Generate README files

## Output Formats

The skill can generate documentation in multiple formats:

- **Inline Documentation**: Rust doc comments (`///` and `//!`)
- **Markdown Files**: README.md, ARCHITECTURE.md, API.md
- **API Reference**: Structured API documentation
- **Architecture Diagrams**: Mermaid diagrams showing relationships
- **Migration Guides**: When updating APIs

## Configuration

You can customize the documentation generation by specifying:

```yaml
scope: "public" # or "all" for private items too
format: "markdown" # or "inline" or "both"
include_examples: true
include_tests: true
include_diagrams: true
depth: "comprehensive" # or "brief" or "detailed"
```

## Best Practices

1. **Document Public APIs First**: Focus on public interfaces that users will interact with
2. **Include Examples**: Show real usage examples, especially for complex APIs
3. **Explain Design Decisions**: Document why certain patterns were chosen
4. **Keep Updated**: Re-run documentation generation after significant changes
5. **Link Related Items**: Cross-reference related types, traits, and functions

## Rust-Specific Features

### Type Documentation

````rust
/// Represents a graph storage backend.
///
/// # Type Parameters
///
/// * `T` - The type of data stored in nodes
///
/// # Examples
///
/// ```rust
/// let storage = MemoryStorage::new();
/// storage.insert_entity("SARAH_CHEN", EntityType::Person)?;
/// ```
pub trait GraphStorage<T> {
    // ...
}
````

### Error Handling Documentation

```rust
/// # Errors
///
/// Returns `StorageError::NotFound` if the entity does not exist.
/// Returns `StorageError::ConnectionFailed` if the database is unavailable.
async fn get_entity(&self, id: &str) -> Result<Entity, StorageError>;
```

### Async Function Documentation

```rust
/// Asynchronously processes a document through the pipeline.
///
/// # Arguments
///
/// * `content` - The document content to process
/// * `metadata` - Optional metadata for the document
///
/// # Returns
///
/// A `PipelineResult` containing extracted entities and relationships.
///
/// # Errors
///
/// Returns `PipelineError` if any processing stage fails.
#[tracing::instrument(skip(self))]
pub async fn process(&self, content: &str, metadata: Option<Metadata>) -> Result<PipelineResult, PipelineError>;
```

## Integration with EdgeQuake

This skill is specifically tuned for EdgeQuake patterns:

- **Crate Organization**: Understands the multi-crate workspace structure
- **Trait Patterns**: Documents storage traits, LLM traits, and query traits
- **Pipeline Patterns**: Documents the document processing pipeline
- **Error Handling**: Documents the custom error types and Result patterns
- **Async Patterns**: Documents tokio-based async operations
- **Testing**: Documents test utilities and integration tests

## Examples

See the `examples/` directory for:

- Full crate documentation generation
- Module documentation
- API reference generation
- Architecture documentation
