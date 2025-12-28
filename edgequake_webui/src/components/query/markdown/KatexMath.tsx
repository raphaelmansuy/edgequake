/**
 * KaTeX Math Component
 * 
 * Renders LaTeX math expressions using KaTeX.
 * Lazy-loaded for performance.
 */
'use client';

import { memo, useMemo } from 'react';
import katex from 'katex';
import 'katex/dist/katex.min.css';

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
