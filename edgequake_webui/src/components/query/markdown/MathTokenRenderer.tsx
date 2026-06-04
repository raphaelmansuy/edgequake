/**
 * Renders custom marked math tokens via KaTeX (block + inline).
 * @implements UI-DRY-008 — single renderer used by block and inline token trees
 */
"use client";

import type { Token } from "marked";
import { lazy, Suspense } from "react";
import { isBlockMathToken, mathContentFromToken } from "./utils/math-token";

const KatexMath = lazy(() => import("./KatexMath"));

function MathInlineFallback({ text }: { text: string }) {
  return (
    <code className="rounded bg-muted px-1 py-0.5 font-mono text-sm">{text}</code>
  );
}

function MathBlockFallback() {
  return (
    <div className="my-4 flex justify-center animate-pulse">
      <div className="h-12 w-48 rounded bg-muted-foreground/20" />
    </div>
  );
}

export interface MathTokenRendererProps {
  token: Token;
}

export function MathTokenRenderer({ token }: MathTokenRendererProps) {
  const block = isBlockMathToken(token.type);
  const math = mathContentFromToken(token);

  return (
    <Suspense
      fallback={block ? <MathBlockFallback /> : <MathInlineFallback text={math} />}
    >
      <KatexMath math={math} block={block} />
    </Suspense>
  );
}
