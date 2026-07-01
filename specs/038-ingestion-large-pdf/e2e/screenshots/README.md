# SPEC-038 E2E Screenshots

Captured by Playwright specs under `edgequake_webui/e2e/`.

| ID | Scenario | File | Spec |
| -- | -------- | ---- | ---- |
| 01 | Documents page before large PDF upload | `01-documents-before-upload.png` | `spec038-large-pdf-admission.spec.ts` |
| 02 | Admission dialog — EdgeParse recommended (603 pages) | `02-admission-dialog-edgeparse-recommended.png` | `spec038-large-pdf-admission.spec.ts` |
| 03 | After confirm — upload wired with `edgeparse` parser | `03-upload-progress-after-confirm.png` | (manual / prior capture) |
| 04 | Vision parser slowdown warning (250 pages) | `04-vision-slowdown-warning.png` | `spec038-large-pdf-admission.spec.ts` |
| 05 | Honest byte progress — 512 KB PDF (no admission) | `05-upload-byte-progress.png` | `spec038-upload-progress.spec.ts` |
| 06 | Admission confirm → transfer progress label | `06-admission-upload-progress.png` | `spec038-upload-progress.spec.ts` |

Capture command:

```bash
cd edgequake_webui
PLAYWRIGHT_BASE_URL=http://localhost:3000 pnpm exec playwright test \
  e2e/spec038-large-pdf-admission.spec.ts \
  e2e/spec038-upload-progress.spec.ts
```
