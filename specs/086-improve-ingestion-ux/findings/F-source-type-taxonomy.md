# F-source-type-taxonomy — Markdown labeled as text/file

> **Finding ID**: `ux086_source_type`  
> **Status**: FIXED  
> **Wave**: 3  
> **Laws**: LAW-25  
> **Verify**: `ux086_v_source_markdown`

---

## 1. Symptom

`.md` uploads are classified as FE `source_type: "text"`; multipart backend may store `"file"`. JSON text upload may use `"markdown"`. Badges, filters, and skip-converting logic become inconsistent.

---

## 2. Evidence (code is law)

| Path | Symbol / lines | Observation |
|------|----------------|-------------|
| `edgequake_webui/src/lib/upload/file-kind.ts` | `classifyUploadFile` | `.md` → `"text"` kind |
| `edgequake_webui/src/lib/upload/perform-file-upload.ts` | (~111–130) | Non-pdf/image → `source_type: "text"`, `isPdf: false` |
| `edgequake_webui/src/lib/upload/__tests__/file-kind.test.ts` | notes.md | Expects `"text"` |
| API `text_upload.rs` | JSON admit | Often `source_type: "markdown"` |
| `multimodal_admission.rs` | resolve upload content | Non-image default `"file"` |

Converting skip today keys on `!= pdf`, so functional skip usually works — taxonomy still wrong for UX/AI.

---

## 3. Root cause

Upload routing collapsed “not PDF/image” into a single `text` bucket. Backend admission paths disagree on vocabulary. No single pin for markdown.

---

## 4. Fix (SOLID/DRY)

- Pin: `.md` / `text/markdown` → `source_type: "markdown"` FE + API.  
- `.txt` → `"text"` (document in 06-contract-pins).  
- Update tests that expect `"text"` for `.md`.  
- Chunk registry continues to use mime/filename; source_type becomes SSOT for UI skip labels.

---

## 5. Edge cases

- `file.md` with wrong mime — extension wins.  
- Reprocess preserves source_type.  
- Existing KV rows with `"file"` — mapper treats as non-pdf (skip converting).

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  pnpm exec vitest run src/lib/upload/__tests__/file-kind.test.ts \
    src/lib/upload/__tests__/perform-file-upload.test.ts
Result: PASS — .md → source_type markdown (FE + multimodal admit)
```
