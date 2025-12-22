# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) documenting significant architectural decisions made during the EdgeQuake project.

## What is an ADR?

An Architecture Decision Record captures an important architectural decision along with its context and consequences. ADRs provide a historical record of why certain technical choices were made.

## ADR Index

| ADR                                            | Title                                  | Status   | Date    |
| ---------------------------------------------- | -------------------------------------- | -------- | ------- |
| [ADR-0001](0001-rust-for-backend.md)           | Use Rust for Backend Implementation    | Accepted | 2024-01 |
| [ADR-0002](0002-modular-crate-architecture.md) | Modular Crate Architecture             | Accepted | 2024-01 |
| [ADR-0003](0003-axum-web-framework.md)         | Axum as Web Framework                  | Accepted | 2024-01 |
| [ADR-0004](0004-trait-based-storage.md)        | Trait-Based Storage Abstraction        | Accepted | 2024-02 |
| [ADR-0005](0005-async-openai-integration.md)   | Async OpenAI for LLM Integration       | Accepted | 2024-02 |
| [ADR-0006](0006-graph-centric-knowledge.md)    | Graph-Centric Knowledge Representation | Accepted | 2024-02 |

## ADR Template

```markdown
# ADR-XXXX: Title

## Status

Proposed | Accepted | Deprecated | Superseded

## Context

What is the issue that we're seeing that is motivating this decision or change?

## Decision

What is the change that we're proposing and/or doing?

## Consequences

What becomes easier or more difficult to do because of this change?
```

## How to Contribute

1. Copy the template above
2. Use the next available number (ADR-XXXX)
3. Fill in all sections
4. Submit a PR with the new ADR
5. Update this README with the new entry
