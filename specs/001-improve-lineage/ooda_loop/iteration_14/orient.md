# Analysis - Iteration 14

## Approach
Follow the existing SDK patterns: resource structs with methods that call `self.client.get()`.
Add types in operations.rs alongside existing lineage types. Keep all fields Optional
with `#[serde(default)]` for maximum compatibility.

## Risk: Low
Pure additions to SDK — no existing methods or types changed.
