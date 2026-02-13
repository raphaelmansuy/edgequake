# Analysis - Iteration 18

## Gap: No API reference docs for lineage endpoints

Developers and SDK users need a clear, standalone API reference to understand request/response schemas without reading Rust source code.

## Approach: REST API Reference Document

- Document all 7 lineage endpoints with path params, response schemas, and examples
- Include SDK usage examples (Rust, TypeScript, Python) for each operation category
- Follow standard REST API documentation format (method, path, params, response, errors)
- Risk: Low — documentation only, no code changes
