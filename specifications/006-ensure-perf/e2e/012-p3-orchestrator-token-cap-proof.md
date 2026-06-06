# E2E Proof 012 — P3 Orchestrator Token Cap Alignment

**Spec:** SPEC-006 P3 · RB-LLM-008  
**Requirement:** Align orchestrator 100k → SOTA 30k  
**Status:** ✅ Verified 2026-06-06

---

## First Principle

> One token budget authority — pipeline RAM scales with `max_token_*`. Drift between orchestrator (100k) and SOTA (30k) was a hidden 3× RAM multiplier.

---

## Code Is Law

| Constant | Value | File |
|----------|-------|------|
| `MAX_ORCHESTRATOR_CONTEXT_TOKENS` | 30000 | `edgequake-core/resource/budget.rs` |
| `EdgeQuakeConfig::default().max_token_for_*` | 30000 | `orchestrator/mod.rs` |

---

## Automated Proof

```bash
cargo test -p edgequake-core orchestrator_context_tokens_align_with_sota --quiet
cargo test -p edgequake-core default_token_budget_matches_resource_ssot --features pipeline --quiet
cargo test -p edgequake-api resource_safety_orchestrator_token_cap_ssot --quiet
```
