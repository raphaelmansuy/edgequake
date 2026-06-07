import katex from "katex";

/** Shared KaTeX options — single source for component + tests. */
export const KATEX_RENDER_OPTIONS = {
  throwOnError: false,
  strict: false,
  trust: true,
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
