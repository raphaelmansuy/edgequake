# OODA-37: Observe

## Observation

Reviewing workspace isolation for document deletion:

- Documents should only be deletable within their workspace context
- Cross-workspace deletion should fail

## Identified Gap

While we have workspace isolation tests for uploads, we need specific tests for:

1. Document deletion respects workspace boundaries
2. Deleting document in workspace A doesn't affect workspace B
3. Document IDs are unique per workspace (or globally?)

## Evidence

Search for existing workspace deletion tests shows limited coverage.
