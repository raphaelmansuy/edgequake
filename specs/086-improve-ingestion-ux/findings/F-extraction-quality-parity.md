# F-extraction-quality-parity — Density, not raw entity equality

> **Finding ID**: `ux086_extract_quality`  
> **Status**: FIXED  
> **Wave**: 3  
> **Laws**: LAW-27  
> **Verify**: `ux086_v_density_gate`, `ux086_e_batch_mixed`

---

## 1. Symptom

Users compare Completed entity counts (e.g. PDF ~3922 vs MD ~35) and conclude Markdown ingestion quality is broken. Feedback poverty amplifies distrust. Some of the gap is real (chunk geometry / content length); some is misread metrics.

---

## 2. Evidence (code is law)

| Path | Observation |
|------|-------------|
| Shared Insert extract path | Same LLM schema after text normalize |
| `ChunkStrategy::Markdown` vs `Pdf` | Different chunk geometry / section breadcrumbs |
| Section context prepend in extractor | Helps MD when headings present |
| UI Cost/Entities columns | Absolute counts only — no density |

FIXED = shared pipeline + checked-in golden-pair fixture + density/section gate.

---

## 3. Root cause

Product surfaces expose **vanity absolute counts** without normalizing for content size or structure. Format-agnostic quality requires comparable proxies (LAW-27), not forcing draft MD ≈ full PDF.

---

## 4. Fix (SOLID/DRY)

- Golden-pair harness: chars, chunks, entities, relationships, section%, cost.  
- Gate: density band + section coverage floor when headings exist (pin thresholds when first baseline run lands).  
- Optional UI affordance later: “entities per 1k chars” in details — not required for Wave 3 gate.  
- Ensure `source_type: markdown` so strategy selection is explicit (`ux086_source_type`).

---

## 5. Edge cases

- Empty/near-empty MD — density undefined; expect low entities, not failure.  
- MD that is PDF convert output vs human draft — document which golden pair.  
- Large-doc profile disabling gleaning — apply equally by size, not format.

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  python3 scripts/ingestion_density_gate.py \
    --fixture specs/086-improve-ingestion-ux/fixtures/density-golden-pair-v1.json
Result: PASS
  md_density=4.0000 entities/1k chars
  pdf_density=6.1111 entities/1k chars
  ratio=0.6545 (floor=0.25)
  md_section_pct=72.0 (floor=50.0)
Fixture: density-golden-pair-v1 (synthetic baseline; replace with production corpus when available)
```
