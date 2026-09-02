# Lens — UX / UI

## Surfaces

| Surface | Change |
|---------|--------|
| Langfuse Cloud / self-hosted UI | Third-party; no EdgeQuake CSS. Benefit: full Output panel |
| EdgeQuake Settings → Langfuse card | Optional one-line honesty: “Generation I/O is exported in full (1 MiB safety cap)” |
| Open in Langfuse / session links | Unchanged (SPEC-124) |

## Honesty rules

- Never show secrets in Settings.
- Do not scrape Langfuse DOM in Playwright as the acceptance gate — use API GET.
- If `io_complete=false` appears in metadata, operators can see overflow was honest.

## Non-goals

- Redesign Langfuse observation layout.
- In-app full-trace viewer duplicating Langfuse.

## Cross-refs

- UX UI spec: [../06-ux-ui-spec.md](../06-ux-ui-spec.md)
- PO: [001-product-owner.md](001-product-owner.md)
