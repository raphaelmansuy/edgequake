# 001 — LaTeX markdown rendering (KaTeX)

**Status:** Implemented in `StreamingMarkdownRenderer` via `configure-marked` math extensions and `KatexMath`.

## Delimiters supported

| Style | Example | Token |
|-------|---------|-------|
| Dollar inline | `$E=mc^2$` | `math_inline` |
| Dollar block | `$$...$$` | `math_block` |
| LaTeX paren inline | `\( \alpha \)` | `math_paren_inline` |
| LaTeX bracket block | `\[ \sum \]` | `math_bracket_block` |

## DRY / SOLID (UI-DRY-008)

| Module | Responsibility |
|--------|----------------|
| `utils/math-marked-extensions.ts` | Tokenize all delimiter styles (one factory) |
| `utils/math-token.ts` | Parse math content from tokens |
| `utils/katex-render.ts` | KaTeX HTML (shared by component + tests) |
| `MathTokenRenderer.tsx` | React render path (block + inline) |
| `KatexMath.tsx` | Present KaTeX HTML |

## Unit tests

- `edgequake_webui/src/components/query/markdown/__tests__/configure-marked-math.test.ts`
- `edgequake_webui/src/components/query/markdown/__tests__/KatexMath.test.ts`

## E2E proof

1. Open `/e2e-fixtures/markdown-latex` (dev-only fixture; no auth/API).
2. Assert `.katex` nodes (≥4) and no raw `$...$` in body text.
3. Capture screenshots under `e2e/screenshots/`.

The same `StreamingMarkdownRenderer` pipeline is used on query responses and document detail (`ContentRenderer`, `MarkdownViewer`).
