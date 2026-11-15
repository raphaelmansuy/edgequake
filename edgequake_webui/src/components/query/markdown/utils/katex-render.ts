import katex from "katex";
import { sanitizeKatexHtml } from "./sanitize-katex-html";

/**
 * Shared KaTeX options — single source for component + tests.
 *
 * `trust: false` is deliberate. With `trust: true`, KaTeX honours `\href` and
 * `\includegraphics`, so an expression arriving from a document chunk, a model answer or a
 * pasted query can emit `<a href="javascript:…">` or fetch an attacker-chosen URL — a stored
 * XSS sink in the query view. Verified: `\href{javascript:alert(1)}{x}` renders a `javascript:`
 * href when `trust` is true and is neutralised when it is false.
 *
 * `trust: false` is not the only gate. `renderKatexToHtml` runs the markup through
 * `sanitizeKatexHtml` before it is returned, so a trusted-command regression or a future
 * KaTeX HTML bug still cannot inject a `javascript:` anchor. See issue #536.
 *
 * `strict: false` stays: it only relaxes typographic warnings, not security.
 */
export const KATEX_RENDER_OPTIONS = {
  throwOnError: false,
  strict: false,
  trust: false,
  output: "html" as const,
};

/** Render LaTeX to sanitized HTML (inline or display mode). */
export function renderKatexToHtml(math: string, block: boolean): string | null {
  try {
    const html = katex.renderToString(math, {
      ...KATEX_RENDER_OPTIONS,
      displayMode: block,
    });
    return sanitizeKatexHtml(html);
  } catch {
    return null;
  }
}
