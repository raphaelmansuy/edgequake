# EdgeQuake Examples

This directory contains example applications demonstrating EdgeQuake's capabilities.

## Examples

### Basic RAG (`basic_rag.rs`)

Demonstrates the core RAG functionality:

- Setting up storage adapters (KV, Vector, Graph)
- Configuring the text chunker
- Processing documents into chunks
- Storing entities and relationships in the knowledge graph
- Displaying graph statistics

**Run:**

```bash
cargo run --example basic_rag
```

### Streaming Query (`streaming_query.rs`)

Demonstrates streaming query capabilities:

- Configuring the query engine
- Executing streaming queries
- Handling streamed response chunks

**Run:**

```bash
cargo run --example streaming_query
```

### Graph Exploration (`graph_exploration.rs`)

Demonstrates knowledge graph capabilities:

- Creating entities (nodes) with properties
- Creating relationships (edges) with properties
- Querying graph structure
- Traversing relationships
- Exploring entity connections

**Run:**

```bash
cargo run --example graph_exploration
```

## Prerequisites

Before running examples, ensure you have:

- Rust 1.78+ installed
- For examples using real LLMs: `OPENAI_API_KEY` environment variable set

## Creating New Examples

When creating new examples:

1. Add the example file to `examples/`
2. Add an entry in `Cargo.toml`:
   ```toml
   [[example]]
   name = "your_example"
   path = "examples/your_example.rs"
   ```
3. Update this README with documentation

## Example Categories

| Category         | Examples            | Description                   |
| ---------------- | ------------------- | ----------------------------- |
| **Core**         | `basic_rag`         | Basic RAG pipeline usage      |
| **Query**        | `streaming_query`   | Query execution and streaming |
| **Graph**        | `graph_exploration` | Knowledge graph operations    |
| **Multi-Tenant** | `multi_tenant`      | Tenant isolation patterns     |

### Multi-Tenant (`multi_tenant.rs`)

Demonstrates multi-tenant data isolation:

- Creating tenant-isolated RAG instances
- Namespaced storage for each tenant
- Ingesting documents per tenant
- Querying within tenant boundaries
- Verifying data isolation between tenants

**Run:**

```bash
cargo run --example multi_tenant
```

## Next Steps

After exploring the examples:

1. Read the [main README](../README.md) for API documentation
2. Explore the crate documentation with `cargo doc --open`
3. Check the test files in each crate for more usage patterns
