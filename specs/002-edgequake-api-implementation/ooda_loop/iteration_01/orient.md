# Iteration 01 — Orient

## Analysis

### Approach: Bottom-Up Foundation

For iteration 01, we build the SDK foundation from bottom-up:

```text
Layer 4: EdgeQuake Client (top-level)           ← this iteration
Layer 3: Resource Classes (auth, documents...)   ← this iteration (scaffolding)
Layer 2: Transport Layer (fetch + middleware)     ← this iteration (core)
Layer 1: Types, Errors, Pagination               ← this iteration (foundation)
```

### Build Tool Selection

| Tool   | Pros                         | Cons                         |
| ------ | ---------------------------- | ---------------------------- |
| tsup   | Zero-config, fast, popular   | Deprecated (use tsdown)      |
| tsdown | Maintained successor of tsup | Very new, less battle-tested |
| tsc    | Standard, reliable           | No bundling, no CJS output   |
| tsup   | Still works, massive usage   | No longer maintained         |

**Decision**: Use `tsup` for iteration 01 — it's stable, widely used, and the design spec
focuses on ESM+CJS dual output which tsup handles perfectly. We can migrate to
tsdown later if needed.

### Test Framework

**Decision**: `vitest` — modern, fast, TypeScript-first, compatible with Jest API.

### Scope for Iteration 01

Given the 131-endpoint mandate, iteration 01 must establish ALL infrastructure
and implement ALL resource classes (even if method bodies are minimal).
This ensures the SDK skeleton is complete and testable.

**Must deliver**:

1. Project scaffolding (package.json, tsconfig, tsup config)
2. Error classes (12 classes)
3. Transport layer (fetch + retry + middleware)
4. Base resource + Pagination
5. Client class with ALL 21 resource namespaces
6. ALL resource implementations with correct method signatures
7. Type definitions for all request/response models
8. Unit tests for: errors, transport, pagination, client creation

### Risk Assessment

| Risk                        | Mitigation                           |
| --------------------------- | ------------------------------------ |
| Type accuracy vs Rust types | Reference handler_types.rs directly  |
| SSE parsing complexity      | Use eventsource-parser library       |
| fetch polyfill for Node <18 | Document requirement, don't polyfill |
| Large file count            | Split by responsibility (SRP)        |
