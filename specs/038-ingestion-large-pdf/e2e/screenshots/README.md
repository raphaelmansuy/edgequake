# SPEC-038 E2E Screenshots

Captured by Playwright specs under `edgequake_webui/e2e/`.

| ID | Scenario | File | Spec |
| -- | -------- | ---- | ---- |
| 01 | Documents page before large PDF upload | `01-documents-before-upload.png` | `spec038-large-pdf-admission.spec.ts` |
| 02 | Admission dialog — EdgeParse recommended (603 pages, Vision resolved) | `02-admission-dialog-edgeparse-recommended.png` | `spec038-large-pdf-admission.spec.ts` |
| 03 | After confirm — upload wired with `edgeparse` parser | `03-upload-progress-after-confirm.png` | `spec038-large-pdf-admission.spec.ts` |
| 04 | Vision parser slowdown warning (250 pages) | `04-vision-slowdown-warning.png` | `spec038-large-pdf-admission.spec.ts` |
| 05 | Honest byte progress — 512 KB PDF (no admission) | `05-upload-byte-progress.png` | `spec038-upload-progress.spec.ts` |
| 06 | Admission confirm → transfer progress label | `06-admission-upload-progress.png` | `spec038-upload-progress.spec.ts` |
| 07 | Silent upload — upload-level EdgeParse selected (603 pages) | `07-silent-upload-edgeparse-selected.png` | unit + `test.fixme` E2E (dev HMR) |
| 08 | Silent upload — workspace default EdgeParse (603 pages) | `08-silent-upload-workspace-edgeparse.png` | unit + `test.fixme` E2E (dev HMR) |

Capture command:

```bash
cd edgequake_webui
PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test \
  e2e/spec038-large-pdf-admission.spec.ts \
  e2e/spec038-upload-progress.spec.ts
```

**Admission gate (REQ-038-12):** Popup appears only when page count ≥ threshold **and** resolved parser is Vision (Upload > Workspace > Server > Vision chain).
