import katex from "katex";

/**
 * Shared KaTeX options — single source for component + tests.
 *
 * `trust: false` is deliberate. With `trust: true`, KaTeX honours `\href` and
 * `\includegraphics`, so an expression arriving from a document chunk, a model answer or a
 * pasted query can emit `<a href="javascript:…">` or fetch an attacker-chosen URL — a stored
 * XSS sink in the query view. Verified: `\href{javascript:alert(1)}{x}` renders a `javascript:`
 * href when `trust` is true and is neutralised when it is false.
 *
 * `strict: false` stays: it only relaxes typographic warnings, not security.
 */
export const KATEX_RENDER_OPTIONS = {
  throwOnError: false,
  strict: false,
  trust: false,
  output: "html" as const,
};

/** Render LaTeX to an HTML string (inline or display mode). */
export function renderKatexToHtml(math: string, block: boolean): string | null {
  try {
    return katex.renderToString(math, {
      ...KATEX_RENDER_OPTIONS,
      displayMode: block,
    });
  } catch {
    return null;
  }
}
