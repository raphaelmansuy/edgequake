# Quick Start Guide - Reverse Documentation Skill

This guide will help you quickly start using the reverse-documentation skill to generate comprehensive documentation for EdgeQuake's codebase.

## Prerequisites

- Access to the EdgeQuake codebase
- AI assistant with file access capabilities
- Basic understanding of Rust and/or TypeScript

## 5-Minute Quick Start

### For Rust Code

**Step 1:** Choose what to document
```
I want to document the edgequake-query crate
```

**Step 2:** The AI will automatically:
- Scan the crate structure
- Analyze all public APIs
- Extract design patterns
- Generate comprehensive documentation

**Step 3:** Review the output
- Check generated doc comments
- Verify examples compile: `cargo test --doc`
- View HTML docs: `cargo doc --open`

### For TypeScript Code

**Step 1:** Choose what to document
```
I want to document the query components in src/components/query
```

**Step 2:** The AI will automatically:
- Scan component files
- Extract prop types
- Identify patterns
- Generate TSDoc comments

**Step 3:** Review the output
- Check generated TSDoc comments
- Verify types: `npx tsc --noEmit`
- Generate docs: `npx typedoc src/`

## Common Use Cases

### Document a Single Crate

```
Generate documentation for the edgequake-storage crate with examples and architecture diagram
```

### Document a Module

```
Document all trait implementations in the storage module
```

### Document React Components

```
Document all components in src/components/workspace with prop examples
```

### Document Custom Hooks

```
Document all custom hooks in src/hooks with usage examples
```

### Update Existing Documentation

```
Review and improve documentation for the QueryEngine struct
```

## Expected Output

### For Rust

✅ `///` doc comments for all public items  
✅ `//!` module documentation  
✅ Working code examples  
✅ Error documentation  
✅ Examples that pass `cargo test --doc`  

### For TypeScript

✅ `/** */` TSDoc comments for all exports  
✅ `@param` and `@returns` tags  
✅ Component prop documentation  
✅ Working usage examples  
✅ Type-safe examples  

## Customization Options

You can customize the documentation generation:

### Comprehensive Mode
```
Generate comprehensive documentation for edgequake-core including:
- All public and private items
- Implementation details
- Performance notes
- Architecture diagrams
```

### Brief Mode
```
Generate brief API documentation for edgequake-core focusing on public interfaces only
```

### With Diagrams
```
Document edgequake-storage with architecture diagram showing all trait relationships
```

## Verification Steps

### Rust
```bash
# Build documentation
cargo doc --no-deps

# Test examples
cargo test --doc

# Check formatting
cargo fmt --check

# Lint
cargo clippy
```

### TypeScript
```bash
# Type check
npx tsc --noEmit

# Generate docs
npx typedoc src/

# Lint
npm run lint

# Build
npm run build
```

## Tips for Best Results

1. **Be Specific**: Mention the exact crate, module, or component
2. **Request Examples**: Always ask for usage examples
3. **Check Coverage**: Review that all public APIs are documented
4. **Verify Examples**: Run tests to ensure examples work
5. **Iterate**: Refine documentation based on feedback

## Common Patterns

### Document an Entire Workspace
```
Generate documentation for all crates in the edgequake workspace
```

### Document API Layer
```
Document all API routes and types in the REST API
```

### Document UI Components
```
Document all shadcn/ui component usage in the workspace management UI
```

## Troubleshooting

**Issue**: Examples don't compile  
**Solution**: Check that all imports are included and types are correct

**Issue**: Missing documentation for some items  
**Solution**: Ensure items are public (have `pub` keyword)

**Issue**: Documentation seems incomplete  
**Solution**: Request comprehensive mode with more details

## Next Steps

After generating documentation:

1. **Review**: Read through generated documentation
2. **Test**: Run `cargo test --doc` or `npm test`
3. **Refine**: Ask for improvements if needed
4. **Integrate**: Commit documentation with code
5. **Maintain**: Update docs when code changes

## Getting Help

If you need help:
1. Check the [full README](./README.md)
2. Review [examples](./examples/)
3. Read language-specific instructions:
   - [Rust instructions](./rust/instructions.md)
   - [TypeScript instructions](./typescript/instructions.md)

## Advanced Usage

### Generate Migration Guide
```
Generate a migration guide from the old storage API to the new async trait-based API
```

### Compare Implementations
```
Document and compare MemoryStorage vs PostgresStorage implementations
```

### Generate Architecture Documentation
```
Create architecture documentation showing the complete RAG pipeline with diagrams
```

## Quality Standards

All generated documentation should:
- ✅ Be accurate and reflect actual code
- ✅ Include working examples
- ✅ Follow language conventions
- ✅ Be maintainable and clear
- ✅ Cross-reference related items

## Feedback

The skill continuously improves based on usage. If you encounter issues or have suggestions, note them for skill improvements.

---

**Ready to start?** Just describe what you want to document in natural language, and the AI assistant will handle the rest!
