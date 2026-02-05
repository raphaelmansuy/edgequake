# Decide – OODA-22: Implement Layout Confidence Calculation

## Decision

Replace hardcoded `confidence: 0.9` with a calculated value based on:

1. Reading order coverage (50% weight)
2. Column detection confidence (30% weight)
3. Region detection quality (20% weight)

## Implementation

Add a private method `calculate_confidence()` to `LayoutAnalyzer` and call it in `analyze()`.

## Code Changes

In `layout/mod.rs`:

1. Add the `calculate_confidence` method
2. Replace `confidence: 0.9` with a call to the new method
3. Add WHY comment explaining the confidence factors
