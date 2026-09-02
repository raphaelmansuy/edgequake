# 06 — UX / UI Spec

## Operator journey

1. Run a Mix/Local query that returns a long answer.
2. Open Settings → Langfuse → Open in Langfuse (or session link).
3. Open the `generate-answer` (or root) observation.
4. Input/Output show the full text; scrollable in Langfuse UI.
5. Tail marker / last section is present (no mid-word cut).

## EdgeQuake Settings (optional copy)

If the Langfuse card mentions I/O:

> Generation and query answers are exported in full to Langfuse (safety cap 1 MiB per field). Secrets are redacted.

No new form fields. Env-only ceiling (`EDGEQUAKE_LANGFUSE_IO_MAX_BYTES`).

## Accessibility / honesty

- Do not claim “full I/O” in UI if export is disabled.
- Overflow: metadata `io_complete=false` is visible in Langfuse metadata panel.

## Out of scope

Playwright against Langfuse Cloud DOM.

## Cross-refs

- UX lens: [05-lenses/004-ux-ui.md](05-lenses/004-ux-ui.md)
- Acceptance: [10-acceptance.md](10-acceptance.md)
