# LENS — AI Engineer

**Job:** define format-agnostic extraction quality without false equality of entity counts.  
**Cites:** LAW-27 · `ux086_extract_quality` · `ux086_source_type` · chunk/extract pipeline

---

## 1. What is the same today

| Concern | Reality |
|---------|---------|
| Post-normalize path | Shared `TaskType::Insert` / text_insert |
| LLM extract schema | Same JSON entity/relationship prompts |
| Gleaning / merge knobs | Shared (large-doc profile may differ by size) |

---

## 2. What legitimately differs

| Concern | Markdown | PDF (after convert) |
|---------|----------|---------------------|
| Chunk strategy | `ChunkStrategy::Markdown` (headings + breadcrumbs) | `ChunkStrategy::Pdf` (page markers) |
| Section context | Prepended when `chunk.section` set | Often absent unless converted MD has headings |
| Page boundaries | N/A | `<!-- edgequake-page:N -->` |
| Vision enrichment | Image uploads only | Converting + optional figure analyze |
| Content length | Draft `.md` may be tiny vs full PDF | Full paper |

**Implication:** Absolute entity counts (e.g. 35 vs 3922) are **not** a format bug by themselves.

---

## 3. Quality parity definition (LAW-27)

| Metric | Definition | Pass idea |
|--------|------------|-----------|
| **Density** | `entities / max(1, chars/1000)` | Golden MD vs PDF of same paper within band (e.g. ratio ≥ 0.4 after length normalize) — pin exact threshold in finding when harness lands |
| **Section coverage** | % chunks with non-empty section for heading-rich MD | ≥ floor when `#` headings exist |
| **Pipeline parity** | Same stage sequence minus converting | Progress timeline skip converting |
| **Failure honesty** | Partial failure counts surfaced | 048/057 inherit |

---

## 4. Source type correctness

`.md` must admit as `source_type: "markdown"` so:

- Converting skip is explicit  
- Chunk registry picks Markdown strategy via mime/filename **and** source_type  
- UI badges/filters do not mislabel as generic `file`/`text`

Finding: `ux086_source_type`.

---

## 5. Experiments / harness (Wave 3)

1. Golden pair: same arXiv paper PDF + markdown export/draft.  
2. Record: chars, chunks, entities, relationships, section%, cost.  
3. Gate fails only on **density cliff** or missing section breadcrumbs when headings present — not on absolute entity equality.  
4. Optional: compare MD-from-PDF-convert vs original MD for structure loss.

---

## 6. Non-goals

- Forcing MD drafts to match PDF entity totals  
- Separate extract model for markdown  
- Changing LightRAG schema in this pack  

---

## 7. Acceptance for AI track

| ID | Gate |
|----|------|
| AI-086-01 | `ux086_v_source_markdown` |
| AI-086-02 | `ux086_v_density_gate` |
| AI-086-03 | Documented narrative in UI/docs: “Entity count scales with content” |
