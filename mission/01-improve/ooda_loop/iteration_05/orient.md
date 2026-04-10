# Orient

- The user goal is not just green tests; it is truthful, reproducible behavior that does not depend on hidden machine state.
- The strongest current signal is test nondeterminism from ambient provider environment variables, not a verified production defect in query execution.
- Restoring the original strict assertions is necessary, but doing only that would risk reintroducing flakiness on machines with different provider env.
- The shared test harness already has the right primitive for deterministic provider tests, so the smallest real fix is to make `e2e_query_http_workspace.rs` explicitly clear provider-detection env before building state/router instances.
- Verification must cover both cases: shell with `OPENAI_API_KEY` present and test process with provider env forcibly unset.# Orient — Iteration 05

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`

## First Principles Analysis

**Principle**: Every `.unwrap()` in production code should either:
1. Be provably infallible with a `// SAFETY:` comment, or
2. Be replaced with `.expect("reason")` explaining the invariant, or
3. Use proper `?` error propagation.

For `serde_json::to_value()` on `#[derive(Serialize)]` structs with only primitive/String fields,
serialization is infallible. Using `.expect()` documents this invariant at the call site.

## Approach

Replace bare `.unwrap()` → `.expect("WHY: <struct> fields are all serializable primitives")`
in production handler code. This costs zero runtime but improves maintainability.
