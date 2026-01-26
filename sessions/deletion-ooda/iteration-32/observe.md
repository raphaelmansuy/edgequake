# OODA-32 Observe: Error Recovery Test

## Gap Identified

No tests for partial failure scenarios:

- What if graph deletion fails after KV deletion?
- What if vector deletion fails?
- Is state left consistent?

## Current Coverage

- Happy path: ✅
- Not found: ✅
- Partial failure: ❌

## Approach

With in-memory storage, it's hard to inject failures.
Instead, test that deletion response includes all metrics
so callers can verify completeness.

## Action: Add response verification tests
