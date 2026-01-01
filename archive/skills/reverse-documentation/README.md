# Reverse Documentation Skill

A comprehensive skill for automatically generating documentation by analyzing existing codebases in Rust and TypeScript.

## Overview

This skill enables AI assistants to understand and document codebases by analyzing code structure, patterns, and relationships. It's specifically tuned for the EdgeQuake project architecture but can be adapted to any Rust or TypeScript codebase.

## Languages Supported

- **Rust**: Comprehensive support for Rust codebases including traits, async patterns, and error handling
- **TypeScript**: Full support for TypeScript/React projects including components, hooks, and Next.js patterns

## Features

### Rust Documentation

- Module structure and organization
- Trait definitions and implementations
- Struct, enum, and type documentation
- Function signatures with error handling
- Async/await patterns
- Generic types and lifetimes
- Cargo workspace structure
- Integration tests and examples

### TypeScript Documentation

- React component documentation
- Custom hooks documentation
- TypeScript interfaces and types
- Props and event handler documentation
- Next.js pages and API routes
- State management patterns
- Generic components
- shadcn/ui component usage

## Project Structure

```
skills/reverse-documentation/
├── README.md                          # This file
├── rust/
│   ├── skill.md                      # Rust skill description
│   └── instructions.md               # Rust-specific instructions
├── typescript/
│   ├── skill.md                      # TypeScript skill description
│   └── instructions.md               # TypeScript-specific instructions
└── examples/
    ├── rust-crate-documentation.md   # Example: Document a Rust crate
    ├── rust-module-documentation.md  # Example: Document a Rust module
    ├── ts-component-documentation.md # Example: Document React components
    └── ts-hook-documentation.md      # Example: Document custom hooks
```

## Usage

### Quick Start

To invoke this skill, simply describe what you want to document:

**Rust Examples:**

```
Generate comprehensive documentation for the edgequake-storage crate
```

```
Document all trait implementations in the graph storage module
```

```
Create API documentation for the LLM provider interface
```

**TypeScript Examples:**

```
Generate documentation for all React components in src/components
```

```
Document the custom hooks in the src/hooks directory
```

```
Create API documentation for the workspace management types
```

### Advanced Usage

You can customize the documentation generation:

```
Document the edgequake-core crate with:
- Comprehensive depth
- Include all examples
- Generate architecture diagrams
- Include internal implementation details
```

```
Document React components in src/components with:
- Include prop examples
- Generate Storybook stories
- Include usage examples
- Add visual component hierarchy
```

## Documentation Standards

### Rust

Follows Rust documentation standards:

- Uses `///` for item documentation
- Uses `//!` for module documentation
- Includes `# Examples`, `# Errors`, `# Panics`, `# Safety` sections
- Provides working code examples
- Documents all public APIs

### TypeScript

Follows TSDoc standards:

- Uses JSDoc-style `/** */` comments
- Includes `@param`, `@returns`, `@example` tags
- Documents all component props
- Provides TypeScript type examples
- Shows realistic usage patterns

## Workflow

When you invoke this skill, the AI assistant will:

1. **Discover**: Scan the codebase to find files and understand structure
2. **Analyze**: Parse code to extract types, functions, and patterns
3. **Understand**: Identify design patterns and relationships
4. **Generate**: Create comprehensive documentation
5. **Validate**: Verify examples compile and documentation is complete

## Integration with EdgeQuake

This skill is specifically designed to work with EdgeQuake's architecture:

### Rust Integration

- Understands the multi-crate workspace structure
- Documents trait-based storage abstraction
- Recognizes LLM provider patterns
- Documents async pipeline processing
- Understands graph data structures

### TypeScript Integration

- Documents Next.js 15 App Router patterns
- Understands shadcn/ui component composition
- Documents SWR data fetching patterns
- Recognizes form handling with react-hook-form
- Documents workspace multi-tenancy patterns

## Quality Standards

All generated documentation must:

✅ Be accurate and reflect actual code behavior  
✅ Include working code examples  
✅ Document all public APIs  
✅ Follow language-specific conventions  
✅ Provide helpful context and explanations  
✅ Cross-reference related items  
✅ Be maintainable and easy to update

## Best Practices

1. **Start with Public APIs**: Focus on user-facing interfaces first
2. **Include Examples**: Always show how to use the code
3. **Explain Patterns**: Document design decisions and patterns
4. **Link Related Items**: Cross-reference related types and functions
5. **Keep Updated**: Regenerate documentation after significant changes
6. **Test Examples**: Ensure all code examples compile and run

## Tools and Commands

### Rust

```bash
# Generate HTML documentation
cargo doc --no-deps --open

# Test documentation examples
cargo test --doc

# Check for broken doc links
cargo install cargo-deadlinks
cargo deadlinks

# Format code
cargo fmt

# Lint code
cargo clippy
```

### TypeScript

```bash
# Generate TypeDoc documentation
npx typedoc src/

# Type check
npx tsc --noEmit

# Lint
npm run lint

# Run Storybook
npm run storybook
```

## Examples

See the `examples/` directory for complete examples:

- [Rust Crate Documentation](examples/rust-crate-documentation.md)
- [Rust Module Documentation](examples/rust-module-documentation.md)
- [TypeScript Component Documentation](examples/ts-component-documentation.md)
- [TypeScript Hook Documentation](examples/ts-hook-documentation.md)

## Advanced Features

### Architecture Diagrams

The skill can generate Mermaid diagrams showing:

- Module dependencies
- Component hierarchies
- Data flow
- Trait relationships
- Type hierarchies

Example:

```
Generate documentation for edgequake-storage with architecture diagram showing all trait implementations and storage backends
```

### Migration Guides

When APIs change, generate migration guides:

```
Generate a migration guide from the old storage API to the new async trait-based API
```

### API Comparison

Compare different implementations:

```
Document and compare the MemoryStorage and PostgresStorage implementations
```

## Customization

You can customize documentation generation by specifying:

```yaml
# Documentation configuration
scope: public # or "all" for private items
format: markdown # or "inline" or "both"
include_examples: true
include_diagrams: true
depth: comprehensive # or "brief" or "detailed"
target_audience: developers # or "maintainers" or "contributors"
```

## Contributing

To extend this skill:

1. Add language-specific patterns to instructions
2. Update examples with new use cases
3. Add templates for common documentation types
4. Improve pattern recognition
5. Add support for new frameworks

## Troubleshooting

### Common Issues

**Documentation not generating:**

- Ensure files are accessible
- Check file permissions
- Verify syntax is valid

**Examples don't compile:**

- Test examples before including
- Ensure all imports are present
- Verify types are correct

**Missing documentation:**

- Check if items are public
- Verify exports are correct
- Ensure items are reachable

## Support

For issues or questions:

1. Check the language-specific instructions
2. Review the examples directory
3. Consult the EdgeQuake documentation
4. File an issue with reproduction steps

## License

This skill follows the EdgeQuake project license.

## Changelog

### Version 1.0.0 (2025-12-24)

- Initial release
- Rust support with trait, async, and error documentation
- TypeScript support with React, hooks, and Next.js
- EdgeQuake-specific patterns and examples
- Comprehensive instructions and examples
