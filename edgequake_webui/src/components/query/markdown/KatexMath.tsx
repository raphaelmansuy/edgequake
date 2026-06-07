/**
 * KaTeX Math Component
 *
 * Renders LaTeX math expressions using KaTeX.
 * Lazy-loaded for performance.
 */
"use client";

import { memo, useMemo } from "react";
import "katex/dist/katex.min.css";
import { renderKatexToHtml } from "./utils/katex-render";

interface KatexMathProps {
  math: string;
  block?: boolean;
  className?: string;
}

export const KatexMath = memo(function KatexMath({
  math,
  block = false,
  className = "",
}: KatexMathProps) {
  const html = useMemo(() => renderKatexToHtml(math, block), [math, block]);

  if (!html) {
    return (
      <code
        className={`rounded bg-muted px-1.5 py-0.5 font-mono text-sm text-red-500 ${className}`}
      >
        {math}
      </code>
    );
  }

  if (block) {
    return (
      <div
        className={`my-4 overflow-x-auto ${className}`}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    );
  }

  return (
    <span
      className={`inline-block ${className}`}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
});

export default KatexMath;
