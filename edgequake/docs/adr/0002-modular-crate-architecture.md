# ADR-0002: Modular Crate Architecture

## Status

Accepted

## Date

2024-01

## Context

EdgeQuake requires a clean separation of concerns to:

1. Enable independent testing of components
2. Allow swapping implementations (e.g., different storage backends)
3. Support selective compilation (only include needed features)
4. Facilitate parallel development by team members
5. Enable future extraction of reusable libraries

A monolithic crate would make these goals difficult to achieve.

## Decision

We organize EdgeQuake as a **Cargo workspace** with 6 core crates:

```
edgequake/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── edgequake-core/     # Core types, errors, utilities
│   ├── edgequake-storage/  # Storage traits and adapters
│   ├── edgequake-llm/      # LLM provider abstractions
│   ├── edgequake-pipeline/ # Document processing pipeline
│   ├── edgequake-query/    # Query engine and strategies
│   └── edgequake-api/      # REST API server
└── src/                    # Main binary entry point
```

### Crate Responsibilities

| Crate                | Purpose                       | Dependencies       |
| -------------------- | ----------------------------- | ------------------ |
| `edgequake-core`     | Types, errors, config         | None (leaf)        |
| `edgequake-storage`  | Storage abstraction           | core               |
| `edgequake-llm`      | LLM/embedding providers       | core               |
| `edgequake-pipeline` | Chunking, extraction, merging | core, storage, llm |
| `edgequake-query`    | Query engine, strategies      | core, storage, llm |
| `edgequake-api`      | HTTP endpoints                | all crates         |

### Design Principles

1. **Leaf crate independence**: core has no internal dependencies
2. **Trait-based abstractions**: storage/llm use traits for flexibility
3. **Acyclic dependencies**: strictly hierarchical, no circular deps
4. **Feature flags**: optional features in each crate

## Consequences

### Positive

- **Fast incremental builds**: Only recompile changed crates
- **Clear ownership**: Each crate has defined responsibility
- **Easy testing**: Unit test each crate in isolation
- **Flexible deployment**: Compile only needed crates
- **Parallel development**: Teams work on different crates

### Negative

- **More files to manage**: 6 Cargo.toml files vs 1
- **Version coordination**: Workspace-level versioning required
- **Cross-crate changes**: Some changes touch multiple crates
- **Learning curve**: Understanding crate boundaries

### Mitigations

- Workspace-level dependency management (`[workspace.dependencies]`)
- CI checks for dependency violations
- Clear documentation of crate boundaries
- Shared workspace settings for consistency
