# SUMMARY — 2026-09-29-demo-query-knee

## Result: PASS (honest cold vs cache)

- Tenant `…0002` / workspace `…0003` on https://demo.edgequake.com
- LLM path for answers: **mistral / mistral-small-latest**
- Cold Q&A: ~7–14s total, 100% success
- C=2/C=4: answer-cache short-circuits (generation_time_ms=0) — not cold capacity
- Knee: **not reached** within concurrency 1–4

See summary.json analysis for business-safe claims.
