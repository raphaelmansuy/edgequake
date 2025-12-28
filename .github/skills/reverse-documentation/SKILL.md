---
name: reverse-documentation
description: Automatically generate comprehensive documentation for Rust and TypeScript codebases by analyzing code structure, patterns, and relationships. Supports trait-based patterns, async operations, React components, and Next.js applications.
license: Proprietary (repository internal)
compatibility: Works with Rust (1.70+), TypeScript (5.0+), and Node.js (18+)
metadata:
  repo: raphaelmansuy/edgequake
  area: documentation-generation
  languages:
    - Rust
    - TypeScript
  patterns:
    - Multi-crate workspace
    - Trait-based abstraction
    - Async/await patterns
    - React components and hooks
    - Next.js App Router
---

# Reverse Documentation Skill

Automatically generate comprehensive documentation for Rust and TypeScript codebases by analyzing existing code structure, patterns, and relationships.

## When to use

Use this skill when you need to:

- Generate comprehensive API documentation for Rust crates
- Document React components and custom hooks
- Create architecture documentation from code analysis
- Generate migration guides for API changes
- Create README and implementation guides
- Document design patterns and relationships
- Build code examples and tutorials
- Cross-reference related functionality

## Core concepts

### Rust Documentation

This skill understands and documents:

- **Module Organization**: Workspace structure, crate dependencies, feature flags
- **Types & Traits**: Struct, enum, and trait definitions with full type signatures
- **Error Handling**: Result types, custom error types, error propagation patterns
- **Async Patterns**: Tokio-based async functions, futures, task spawning
- **Generic Types**: Type parameters, bounds, and lifetime annotations
- **Implementation Details**: Methods, associated functions, trait implementations
- **Testing**: Unit tests, integration tests, documentation examples

### TypeScript Documentation

This skill understands and documents:

- **Module Structure**: Exports, re-exports, and module dependencies
- **Type Definitions**: Interfaces, types, generics, and utility types
- **React Components**: Functional components, props, state, lifecycle
- **Custom Hooks**: Hook composition, state management, side effects
- **API Integration**: Data fetching, request/response types, error handling
- **State Patterns**: useState, useReducer, Context API, state management libraries
- **Next.js Patterns**: Pages, layouts, API routes, middleware, server components

## Quick start

### For Rust codebases

```
Generate comprehensive documentation for the [crate-name] crate
```

```
Document the trait-based storage abstraction in edgequake-storage
```

```
Create API documentation for all public interfaces with examples
```

### For TypeScript codebases

```
Generate documentation for all React components in src/components
```

```
Document the custom hooks in src/hooks with usage examples
```

```
Create type documentation for the API integration layer
```

## Capabilities

### Rust Capabilities

#### 1. Crate Documentation
- Analyze Cargo.toml and workspace structure
- Extract and document all public types, traits, and functions
- Generate comprehensive README files
- Create architecture diagrams of trait relationships
- Document feature flags and optional dependencies

#### 2. API Reference Generation
- Extract function signatures with parameters and return types
- Document error types and handling patterns
- Include working code examples
- Show generic type usage
- Cross-reference related types

#### 3. Pattern Documentation
- Identify and document design patterns (Builder, Factory, Strategy, etc.)
- Explain async/await patterns and tokio usage
- Document error handling approaches
- Show trait implementations and polymorphism
- Explain generic type usage

#### 4. Example Generation
- Create working code examples for public APIs
- Show error handling patterns
- Demonstrate async operations
- Include edge cases and common pitfalls

### TypeScript Capabilities

#### 1. Component Documentation
- Extract component props with types and defaults
- Document component behavior and event handlers
- Generate prop combinations and variants
- Create Storybook stories
- Show component composition patterns

#### 2. Hook Documentation
- Document hook parameters and return values
- Show hook composition patterns
- Explain dependency arrays
- Include usage examples
- Identify potential performance issues

#### 3. Type Documentation
- Extract and document all exported types and interfaces
- Show generic type parameters
- Document utility type usage
- Create type hierarchy diagrams
- Show API request/response types

#### 4. Architecture Documentation
- Generate data flow diagrams
- Document state management patterns
- Show API integration patterns
- Identify dependency trees
- Document component hierarchies

## Workflow

When you invoke this skill, the AI assistant will:

1. **Discovery Phase**: Scan the codebase to find files and understand structure
2. **Analysis Phase**: Parse code to extract types, functions, patterns, and relationships
3. **Understanding Phase**: Identify design patterns, architectural decisions, and key concepts
4. **Generation Phase**: Create comprehensive documentation in your chosen format
5. **Validation Phase**: Verify examples compile and documentation is complete

## Output formats

### Markdown Documentation
- Module-level README files
- API reference documentation
- Architecture documentation
- Migration guides
- Best practices guides

### Inline Documentation
- Rust doc comments (/// and //!)
- TypeScript JSDoc comments
- Follow language conventions
- Include examples and sections

### Diagrams
- Trait relationship diagrams (Mermaid)
- Component hierarchy diagrams
- Data flow diagrams
- Module dependency graphs

### Storybook Stories (TypeScript)
- Component prop variations
- Interactive examples
- Usage patterns
- Edge cases

## Configuration options

Customize documentation generation:

```yaml
# Scope of documentation
scope: "public"           # or "all" for private items too

# Output format
format: "markdown"        # or "inline" or "both"

# Include sections
include_examples: true
include_tests: true       # Rust only
include_diagrams: true
include_stories: true     # TypeScript only

# Documentation depth
depth: "comprehensive"    # or "brief" or "detailed"

# Target audience
audience: "developers"    # or "maintainers" or "contributors"
```

## Best practices

### Rust Documentation

- ✅ Document all public APIs
- ✅ Include working code examples that compile
- ✅ Show error cases and how to handle them
- ✅ Explain generic type parameters and constraints
- ✅ Document async/await usage and tokio requirements
- ✅ Cross-reference related types and traits

### TypeScript Documentation

- ✅ Document all component props with types
- ✅ Show component prop variations
- ✅ Explain hook return values and side effects
- ✅ Document data flow and state management
- ✅ Include usage examples with actual code
- ✅ Document performance considerations

## EdgeQuake-specific patterns

### Rust

- **Multi-crate Workspace**: `edgequake-core`, `edgequake-storage`, `edgequake-llm`, `edgequake-api`
- **Trait Abstraction**: `GraphStorage`, `LLMProvider`, `StorageAdapter` traits
- **Error Handling**: Custom `StorageError`, `PipelineError` types
- **Async Pipeline**: Document the entity extraction and graph building pipeline
- **Entity Normalization**: Special naming conventions (e.g., "SARAH_CHEN")

### TypeScript

- **Next.js 15 App Router**: Document pages, layouts, and API routes
- **shadcn/ui Components**: Document UI component composition
- **Data Fetching**: SWR hooks and API integration patterns
- **Form Handling**: react-hook-form patterns
- **State Management**: Workspace, query, and document state patterns
- **Streaming**: SSE and streaming response handling

## Examples

### Rust Example

```
Generate comprehensive documentation for the edgequake-storage crate including:
- All trait definitions and implementations
- Storage backend comparison (Memory vs PostgreSQL)
- Error handling patterns
- Async operation patterns
- Integration tests
- Architecture diagram showing trait relationships
```

### TypeScript Example

```
Generate documentation for the edgequake_webui components including:
- All React components with props
- Custom hooks in src/hooks
- API integration types
- Component composition examples
- Storybook stories for all components
- Data flow diagram
```

## Troubleshooting

### Documentation not generating

- ✓ Ensure files are accessible and readable
- ✓ Check file permissions
- ✓ Verify syntax is valid

### Examples don't compile (Rust)

- ✓ Test examples before including them
- ✓ Ensure all imports are present
- ✓ Verify types are correct and in scope

### Missing documentation

- ✓ Check if items are public/exported
- ✓ Verify exports are in correct module
- ✓ Ensure items are accessible from public API

## Related skills

- **makefile-dev-workflow**: Development workflow commands
- **playwright-ux-ui-capture**: UI screenshot capture automation
- **ux-ui-analyze-single-page**: Single page UX analysis

## See also

- [Rust Book - Documentation](https://doc.rust-lang.org/book/ch14-04-installing-binaries.html#distributing-binaries-with-cargo-install)
- [TypeScript Documentation](https://www.typescriptlang.org/docs/)
- [React Documentation](https://react.dev/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
