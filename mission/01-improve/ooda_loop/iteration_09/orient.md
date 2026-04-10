# Orient — Iteration 09
Date: 2026-04-10

## First Principles
An extension trait on `Result<T, E>` that provides `.internal_err("context")` and a helper `parse_uuid(s, label) -> Result<Uuid, ApiError>` would eliminate all 69 duplications.

However, changing 57+ call sites in one iteration is too risky. Better to:
1. Add the helpers
2. Use them in new code going forward
3. Migrate existing code in small batches
