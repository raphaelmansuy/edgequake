# Screenshots — what each one proves

Captured from the running stack (`:3010` UI → `:8090` backend → Postgres + Vertex), against
the real SPEC-182 corpus, not mocked routes. Naming is `<step>-<what>-<theme>-<width>.png`
under `round-02/` (the set the article quotes).

Each image is produced by a test that also asserts the behaviour it shows
([`e2e/spec182/`](../../edgequake_webui/e2e/spec182/)). A screenshot that stops being true
fails a test instead of ageing quietly.

## The set

| File | What it proves |
|---|---|
| `10-documents-corpus` | 20 pictures ingested, each listed under a readable caption — not a filename. Chunks = 1 per picture, the shape of a chunk-only workspace. |
| `11-document-image-detail` | A picture document renders **the picture**, at its real aspect ratio, with zoom and download. The body is not the filename painted as text. |
| `12-document-image-detail` (dark) | The same, in dark theme. Included because a screenshot that only works in one theme is half a screenshot. |
| `13-document-image-detail` (390 px) | The same on a phone. Confirms the viewer reflows rather than cropping the subject out. |
| `20-query-composer-image-capable` | The composer shows **Search by image** and an **Images** filter. Both are gated on `capabilities.embeds_images`, so their presence is the capability claim. |
| `21-query-search-image-attached` | The chosen picture as a chip with a thumbnail and a remove control — the affordance exists before and after the search. |
| `22-query-image-results` | **The money shot.** A picture-only search (empty text) answered with a "Matching images" grid of the pictures retrieval found, each with its citation number and match strength. |
| `22b-query-image-matches-grid` | The grid alone, cropped, for use as an inline figure. |
| `23-query-image-results` (dark) | The same result in dark theme. |
| `24-query-image-results` (390 px) | The same on a phone, where the grid drops to 3 columns instead of overflowing. |
| `26-query-source-pane-picture` | A cited picture opens **as a picture** in the source pane, not as a text pane holding its filename. |

Tiles show a `[n]` citation number and **no percentage**. That is deliberate: on a normal
workspace `score` is an RRF rank score, so any percentage derived from it reads "2%" for a
perfect match. See the last section of [`../api/RESULTS.md`](../api/RESULTS.md).

## Companion figures

| File | What it is |
|---|---|
| [`../images/corpus-contact-sheet.png`](../images/corpus-contact-sheet.png) | All 20 pictures with filenames. Use it to show what the library contains; the article's near-miss argument depends on the reader seeing three dogs and two coffees. |
| [`../images/probes/`](../images/probes/) | The four out-of-corpus probes used for image-to-image search. Not in the library, so a passing result means the vector did the work. |

## Rules these follow

1. **No mock routes.** The API is already proven separately in [`../api/RESULTS.md`](../api/RESULTS.md);
   a mocked screenshot would only prove that React can render JSON someone typed.
2. **Viewport, not full page.** The app scrolls inside containers, so a full-page capture
   stitches those containers at their scroll origin and loses the interesting middle.
3. **Assert before capturing.** Every test waits on a real post-condition — the `<img>` has
   `naturalWidth > 0`, the grid has more than one tile — because "visible" arrives before
   the bytes do, and an early capture once produced a broken-image icon.
4. **No warning banners.** If "No LLM provider is reachable" is on screen, the test fails
   with an actionable message rather than shipping a screenshot of a problem unrelated to
   the feature.

## Regenerating

```bash
# One command does all of it: starts Postgres and the backend, captures, stops everything.
# WHY bundled: this environment reaps detached processes between tool calls, so a
# "start in one step, test in the next" workflow finds dead ports.
bash specs/182-rags/e2e/capture.sh
```

That script expects the corpus tenant to exist already (`api/scope.env`). To build one:

```bash
bash specs/182-rags/e2e/evidence.sh      # seeds the corpus AND verifies 24 checks
bash specs/182-rags/e2e/capture.sh       # then captures from the same workspace
```

`SPEC182_ROUND=<name>` writes to a different folder, so a revision does not overwrite the
set the article quotes. `round-02` is the set the article uses.
