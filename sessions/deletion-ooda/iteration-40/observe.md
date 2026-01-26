# OODA-40: Observe

## Observation

EdgeQuake uses SHA-256 hash for content deduplication (BR0001).
Need to verify:
1. Same content → same hash
2. Different content → different hash
3. Hash is returned in upload response

## Gap

No explicit tests for hash-based deduplication behavior in deletion context.

## Evidence

Looking at upload response for `content_hash` field verification.
