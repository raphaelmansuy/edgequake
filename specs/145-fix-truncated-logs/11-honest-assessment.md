# 11 — Honest Assessment

## What was broken

Silent 512-byte preview on **all** observation I/O made Langfuse generations
unusable for long answers. SPEC-124 treated truncation as a privacy feature;
that was the wrong lever.

## What we keep

- LAW-124-8: no API keys / full configs / dump-all-args
- LAW-124-12: tokens yes, cost never
- LAW-124-20: metadata filter values ≤ 200 chars
- Ingest content Preview; chunking counts-only (SPEC-125)
- Dual export + non-blocking batch

## Residual risks

| Risk | Status |
|------|--------|
| Long prompts increase export payload | Mitigated by 1 MiB ceiling |
| Workspace content in Langfuse | Ops retention / access control |
| Stream assemble race | Record after verified assemble |
| Redaction misses novel secret shapes | Expand denylist as found; do not reintroduce 512 |

## Cross-refs

- WHY: [00-why.md](00-why.md)
- Acceptance: [10-acceptance.md](10-acceptance.md)
