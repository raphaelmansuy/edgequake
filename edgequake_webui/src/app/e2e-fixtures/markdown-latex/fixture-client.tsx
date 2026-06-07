"use client";

import { StreamingMarkdownRenderer } from "@/components/query/markdown";
import { LATEX_MARKDOWN_SAMPLE } from "@/lib/fixtures/latex-markdown-sample";

export function MarkdownLatexFixtureClient() {
  return (
    <div
      className="min-h-screen bg-background p-8"
      data-testid="markdown-latex-fixture"
    >
      <StreamingMarkdownRenderer content={LATEX_MARKDOWN_SAMPLE} />
    </div>
  );
}
