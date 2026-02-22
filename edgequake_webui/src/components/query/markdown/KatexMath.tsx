/**
 * KaTeX Math Component
 * 
 * Renders LaTeX math expressions using KaTeX.
 * Lazy-loaded for performance.
 */
'use client';

import katex from 'katex';
import 'katex/dist/katex.min.css';
import { memo, useMemo } from 'react';

interface KatexMathProps {
  math: string;
  block?: boolean;
  className?: string;
}

export const KatexMath = memo(function KatexMath({
  math,
  block = false,
  className = '',
}: KatexMathProps) {
  const html = useMemo(() => {
    try {
      return katex.renderToString(math, {
        displayMode: block,
        throwOnError: false,
        strict: false,
        trust: true,
        output: 'html',
      });
    } catch (error) {
      console.error('KaTeX render error:', error);
      return null;
    }
  }, [math, block]);

  if (!html) {
    // Fallback to code display on error
    return (
      <code
        className={`rounded bg-muted px-1.5 py-0.5 font-mono text-sm text-destructive ${className}`}
        role="math"
        aria-label={`Math expression: ${math}`}
      >
        {math}
      </code>
    );
  }

  if (block) {
    return (
      <div
        className={`my-4 overflow-x-auto text-foreground [&_.katex]:text-foreground ${className}`}
        role="math"
        aria-label={`Math expression: ${math}`}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    );
  }

  return (
    <span
      className={`inline-block text-foreground [&_.katex]:text-foreground ${className}`}
      role="math"
      aria-label={`Math expression: ${math}`}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
});

export default KatexMath;
