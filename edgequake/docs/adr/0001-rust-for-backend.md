# ADR-0001: Use Rust for Backend Implementation

## Status

Accepted

## Date

2024-01

## Context

The original LightRAG implementation is written in Python, which provides rapid development and easy integration with ML/AI libraries. However, for production deployment we need:

1. **Performance**: RAG systems process large documents and perform many vector operations
2. **Concurrency**: High-throughput API serving with many concurrent connections
3. **Memory Safety**: Reliable production systems without memory leaks
4. **Type Safety**: Catch errors at compile time rather than runtime
5. **Deployment Simplicity**: Single binary deployment without runtime dependencies

Python's GIL (Global Interpreter Lock) limits true parallelism, and its dynamic typing can lead to runtime errors in production.

## Decision

We chose **Rust** as the primary backend language for EdgeQuake because:

1. **Zero-cost abstractions**: Performance comparable to C/C++ with high-level ergonomics
2. **Memory safety without GC**: No garbage collection pauses, predictable latency
3. **Fearless concurrency**: The type system prevents data races at compile time
4. **async/await**: Excellent async runtime (tokio) for high-concurrency HTTP servers
5. **Rich ecosystem**: Quality libraries for HTTP (axum), serialization (serde), and async (tokio)
6. **Single binary**: Easy deployment without managing Python environments
7. **WASM support**: Future potential for edge/browser deployment

### Alternatives Considered

| Language    | Pros                            | Cons                               |
| ----------- | ------------------------------- | ---------------------------------- |
| Python      | Existing codebase, ML libraries | GIL, dynamic typing, memory        |
| Go          | Simple, good concurrency        | Less ergonomic generics, GC pauses |
| C++         | Maximum performance             | Memory safety concerns, complexity |
| Java/Kotlin | Mature ecosystem                | JVM overhead, cold start times     |

## Consequences

### Positive

- **10-50x performance improvement** over Python for CPU-bound operations
- **Predictable latency** without GC pauses
- **Type-safe API contracts** caught at compile time
- **Small deployment footprint** (single binary < 50MB)
- **Strong concurrency guarantees** from the compiler

### Negative

- **Steeper learning curve** for team members new to Rust
- **Longer compile times** compared to interpreted languages
- **Fewer ML-specific libraries** (though we primarily call external LLM APIs)
- **More verbose code** for some operations vs Python
- **Port complexity** - significant effort to port from Python

### Mitigations

- Provide Rust training and documentation
- Use incremental compilation and caching (sccache)
- Call external APIs for LLM operations (minimal ML library needs)
- Develop idiomatic Rust patterns to reduce verbosity
