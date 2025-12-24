# Rust Reverse Documentation Instructions

## Objective

You are a Rust documentation expert tasked with analyzing Rust codebases and generating comprehensive, accurate, and helpful documentation.

## Analysis Process

### 1. Code Discovery

Start by exploring the codebase:

```bash
# Find all Rust source files
fd -e rs

# Identify the crate structure
find . -name "Cargo.toml"

# Understand the workspace structure
cat Cargo.toml
```

### 2. Module Analysis

For each module, identify:

- **Public API Surface**: All `pub` items (functions, structs, enums, traits, type aliases)
- **Private Implementation**: Internal helpers and implementation details
- **Dependencies**: External crates and internal module dependencies
- **Re-exports**: Items re-exported in `lib.rs` or `mod.rs`

### 3. Type Analysis

For each type (struct, enum, trait), extract:

- **Purpose**: What the type represents
- **Fields/Variants**: All public and private members
- **Methods**: Associated functions and methods
- **Trait Implementations**: Which traits are implemented
- **Lifetime Parameters**: Generic lifetimes and their constraints
- **Type Parameters**: Generic types and trait bounds
- **Examples**: Usage examples

### 4. Function Analysis

For each function, document:

- **Purpose**: What the function does
- **Parameters**: Each parameter with type and description
- **Return Type**: What the function returns
- **Errors**: Possible error conditions (for Result types)
- **Panics**: Conditions that cause panics
- **Safety**: For unsafe functions, safety requirements
- **Examples**: Usage examples
- **Complexity**: Time/space complexity if relevant

### 5. Trait Analysis

For traits, document:

- **Purpose**: What the trait abstracts
- **Required Methods**: Methods implementers must provide
- **Provided Methods**: Default implementations
- **Associated Types**: Types that implementers must specify
- **Super Traits**: Trait bounds on the trait itself
- **Implementers**: Known types that implement the trait
- **Examples**: How to implement and use the trait

## Documentation Standards

### Rust Doc Comment Format

Use standard Rust documentation comments:

````rust
/// Short one-line summary.
///
/// Longer description with multiple paragraphs if needed.
///
/// # Examples
///
/// ```rust
/// // Code example
/// ```
///
/// # Errors
///
/// This function returns an error if...
///
/// # Panics
///
/// Panics if...
///
/// # Safety
///
/// (For unsafe functions only)
````

### Module Documentation

````rust
//! Module-level documentation.
//!
//! This module provides...
//!
//! # Examples
//!
//! ```rust
//! use crate::module::Type;
//! ```
````

### Documentation Sections

Use these standard sections in order:

1. Summary (one line)
2. Description (detailed, multiple paragraphs OK)
3. `# Examples` - Working code examples
4. `# Parameters` or `# Arguments` - For functions (optional if obvious)
5. `# Returns` - What the function returns (optional if obvious)
6. `# Errors` - For Result-returning functions
7. `# Panics` - Conditions that cause panics
8. `# Safety` - For unsafe code
9. `# Notes` - Additional information
10. `# See also` - Links to related items

## Code Analysis Techniques

### Using cargo doc

```bash
# Generate documentation to understand structure
cargo doc --no-deps --document-private-items

# Open generated docs
cargo doc --open
```

### Using rust-analyzer

```bash
# Get information about a symbol
rust-analyzer symbols

# Find implementations
rust-analyzer implementations
```

### Using grep/ripgrep

```bash
# Find all trait definitions
rg "^pub trait"

# Find all struct definitions
rg "^pub struct"

# Find all implementations
rg "^impl"
```

## Pattern Recognition

### Common Patterns to Document

1. **Builder Pattern**

   ```rust
   /// Builder for configuring X.
   pub struct XBuilder { /* ... */ }
   ```

2. **Error Types**

   ```rust
   /// Errors that can occur during Y operations.
   #[derive(Debug, thiserror::Error)]
   pub enum YError { /* ... */ }
   ```

3. **Trait Objects**

   ```rust
   /// Dynamic dispatch for Z functionality.
   pub type BoxedZ = Box<dyn Z + Send + Sync>;
   ```

4. **Async Traits**

   ```rust
   /// Async trait for performing W operations.
   #[async_trait]
   pub trait W { /* ... */ }
   ```

5. **Newtype Pattern**
   ```rust
   /// Wrapper around V for type safety.
   pub struct NewV(V);
   ```

## EdgeQuake-Specific Patterns

### Storage Traits

```rust
/// Storage backend for graph data.
///
/// Implementations provide different storage strategies:
/// - `MemoryStorage`: In-memory HashMap-based storage
/// - `PostgresStorage`: PostgreSQL with AGE extension
#[async_trait]
pub trait GraphStorage: Send + Sync {
    // Document each method
}
```

### LLM Traits

```rust
/// Language model provider interface.
///
/// # Implementations
///
/// - `OpenAiProvider`: Uses OpenAI API
/// - `MockProvider`: Mock for testing
#[async_trait]
pub trait LLMProvider {
    // Document methods
}
```

### Pipeline Pattern

```rust
/// Document processing pipeline.
///
/// The pipeline consists of stages:
/// 1. Text chunking
/// 2. Entity extraction
/// 3. Relationship extraction
/// 4. Graph storage
pub struct Pipeline {
    // Document fields
}
```

## Output Requirements

Generate documentation that:

1. **Is Accurate**: Reflects the actual code behavior
2. **Is Complete**: Covers all public APIs
3. **Is Helpful**: Provides examples and explains why, not just what
4. **Is Consistent**: Uses uniform style and terminology
5. **Is Maintainable**: Easy to update as code changes

## Quality Checklist

Before considering documentation complete, verify:

- [ ] All public types have documentation
- [ ] All public functions have documentation
- [ ] All public traits have documentation
- [ ] Examples compile and run
- [ ] Error conditions are documented
- [ ] Panic conditions are documented
- [ ] Unsafe code safety requirements are documented
- [ ] Cross-references between related items exist
- [ ] Module-level documentation exists
- [ ] Crate-level documentation exists (lib.rs)

## Special Considerations

### Generic Types

```rust
/// Container for items of type `T`.
///
/// # Type Parameters
///
/// * `T` - The type of items stored. Must implement `Clone` for duplication.
pub struct Container<T: Clone> {
    items: Vec<T>,
}
```

### Lifetimes

```rust
/// Reference to data with lifetime `'a`.
///
/// # Lifetimes
///
/// * `'a` - The lifetime of the borrowed data.
pub struct DataRef<'a> {
    data: &'a str,
}
```

### Async Functions

```rust
/// Asynchronously fetches data from the source.
///
/// # Async
///
/// This function is async and must be awaited. It uses tokio for async runtime.
///
/// # Cancellation
///
/// The operation can be cancelled by dropping the future.
pub async fn fetch_data(&self) -> Result<Data, Error> {
    // ...
}
```

## Tools and Utilities

### cargo-readme

Generate README from doc comments:

```bash
cargo install cargo-readme
cargo readme > README.md
```

### rustdoc

Check documentation:

```bash
cargo doc --no-deps
```

### cargo-deadlinks

Check for broken doc links:

```bash
cargo install cargo-deadlinks
cargo deadlinks
```

## Continuous Documentation

After generating documentation:

1. Run `cargo doc` to ensure it builds
2. Run `cargo test --doc` to verify examples
3. Check for broken links
4. Review generated HTML documentation
5. Commit documentation with code changes

## Common Mistakes to Avoid

1. ❌ Documenting private implementation details in public docs
2. ❌ Using examples that don't compile
3. ❌ Forgetting to document error conditions
4. ❌ Using vague language like "does stuff"
5. ❌ Not explaining complex algorithms or patterns
6. ❌ Inconsistent terminology across related types
7. ❌ Missing panic conditions
8. ❌ Not documenting unsafe requirements

## Best Practices

1. ✅ Start with a one-line summary
2. ✅ Provide working examples
3. ✅ Explain the "why" not just the "what"
4. ✅ Link to related items with `[Type]` or `[function]`
5. ✅ Use consistent terminology
6. ✅ Document edge cases
7. ✅ Explain performance characteristics when relevant
8. ✅ Keep documentation close to code
