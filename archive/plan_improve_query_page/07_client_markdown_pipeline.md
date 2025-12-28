# Client-Side: Markdown Rendering Pipeline

**Document**: `07_client_markdown_pipeline.md`  
**Created**: 2024-12-27  
**Status**: Draft

---

## 1. Overview

This document provides implementation guidance for rebuilding the markdown rendering pipeline in EdgeQuake WebUI, adopting the token-based approach from open-webui while maintaining React/Next.js architecture.

### Current Issues

| Issue                             | Location                    | Impact                       |
| --------------------------------- | --------------------------- | ---------------------------- |
| Streaming fallback to plain text  | `markdown-renderer.tsx:720` | Broken UX during streaming   |
| Disabled KaTeX                    | `markdown-renderer.tsx:269` | No math equations            |
| Mermaid disabled during streaming | `markdown-renderer.tsx:322` | Diagrams only after complete |
| 60+ regex normalizations          | `markdown-renderer.tsx:644` | Fragile, slow                |

### Solution: Token-Based Rendering

Adopt `marked.js` lexer for tokenization, render each token type independently.

---

## 2. Architecture

### 2.1 Component Hierarchy

```
src/components/query/markdown/
├── index.ts                      # Re-exports
├── StreamingMarkdownRenderer.tsx # Main entry point
├── MarkdownTokens.tsx            # Block-level token renderer
├── MarkdownInlineTokens.tsx      # Inline token renderer
├── tokens/
│   ├── ParagraphToken.tsx
│   ├── HeadingToken.tsx
│   ├── CodeToken.tsx
│   ├── TableToken.tsx
│   ├── ListToken.tsx
│   ├── BlockquoteToken.tsx
│   ├── HorizontalRuleToken.tsx
│   └── HtmlToken.tsx
├── inline/
│   ├── TextToken.tsx
│   ├── StrongToken.tsx
│   ├── EmphasisToken.tsx
│   ├── CodespanToken.tsx
│   ├── LinkToken.tsx
│   ├── ImageToken.tsx
│   ├── KatexToken.tsx
│   └── CitationToken.tsx
├── extensions/
│   ├── katex-extension.ts
│   ├── citation-extension.ts
│   └── mermaid-extension.ts
└── utils/
    ├── streaming-parser.ts
    └── token-helpers.ts
```

### 2.2 Data Flow

```
LLM Stream → Buffer → Lexer (marked.js) → Token[] → TokenRenderer → DOM

┌──────────────┐     ┌─────────────┐     ┌────────────────┐
│ Content      │     │ marked      │     │ MarkdownTokens │
│ (streaming)  │────▶│ .lexer()    │────▶│ (per-type)     │────▶ DOM
└──────────────┘     └─────────────┘     └────────────────┘
       │                                         │
       └── isStreaming ─────────────────────────▶│
           (controls partial display)
```

---

## 3. Core Components

### 3.1 StreamingMarkdownRenderer

Main entry point that handles streaming and tokenization.

```typescript
// src/components/query/markdown/StreamingMarkdownRenderer.tsx
"use client";

import { marked, type Token } from "marked";
import { memo, useMemo, useRef, useEffect } from "react";
import { MarkdownTokens } from "./MarkdownTokens";
import { configureMarked } from "./utils/configure-marked";

interface StreamingMarkdownRendererProps {
  content: string;
  isStreaming?: boolean;
  className?: string;
  enableMath?: boolean;
  enableMermaid?: boolean;
  onSourceClick?: (sourceId: string) => void;
}

// Configure marked once on module load
configureMarked();

export const StreamingMarkdownRenderer = memo(
  function StreamingMarkdownRenderer({
    content,
    isStreaming = false,
    className,
    enableMath = true,
    enableMermaid = true,
    onSourceClick,
  }: StreamingMarkdownRendererProps) {
    const idRef = useRef(`md-${Math.random().toString(36).slice(2, 9)}`);

    // Tokenize content using marked lexer
    const tokens = useMemo(() => {
      if (!content) return [];
      try {
        return marked.lexer(content);
      } catch (error) {
        console.error("Markdown lexer error:", error);
        return [];
      }
    }, [content]);

    return (
      <div className={className}>
        <MarkdownTokens
          id={idRef.current}
          tokens={tokens}
          done={!isStreaming}
          enableMath={enableMath}
          enableMermaid={enableMermaid}
          onSourceClick={onSourceClick}
        />
      </div>
    );
  }
);

export default StreamingMarkdownRenderer;
```

### 3.2 MarkdownTokens (Block-Level)

Renders block-level tokens with switch dispatch.

```typescript
// src/components/query/markdown/MarkdownTokens.tsx
"use client";

import type { Token, Tokens } from "marked";
import { memo } from "react";
import { MarkdownInlineTokens } from "./MarkdownInlineTokens";
import { CodeToken } from "./tokens/CodeToken";
import { TableToken } from "./tokens/TableToken";
import { ListToken } from "./tokens/ListToken";

interface MarkdownTokensProps {
  id: string;
  tokens: Token[];
  done?: boolean;
  enableMath?: boolean;
  enableMermaid?: boolean;
  onSourceClick?: (sourceId: string) => void;
}

export const MarkdownTokens = memo(function MarkdownTokens({
  id,
  tokens,
  done = true,
  enableMath = true,
  enableMermaid = true,
  onSourceClick,
}: MarkdownTokensProps) {
  return (
    <>
      {tokens.map((token, idx) => {
        const tokenId = `${id}-${idx}`;

        switch (token.type) {
          case "heading": {
            const HeadingTag = `h${(token as Tokens.Heading).depth}` as const;
            return (
              <HeadingTag key={tokenId} dir="auto">
                <MarkdownInlineTokens
                  id={`${tokenId}-h`}
                  tokens={(token as Tokens.Heading).tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </HeadingTag>
            );
          }

          case "paragraph":
            return (
              <p key={tokenId} dir="auto">
                <MarkdownInlineTokens
                  id={`${tokenId}-p`}
                  tokens={(token as Tokens.Paragraph).tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </p>
            );

          case "code":
            return (
              <CodeToken
                key={tokenId}
                id={tokenId}
                token={token as Tokens.Code}
                done={done}
                enableMermaid={enableMermaid}
              />
            );

          case "table":
            return (
              <TableToken
                key={tokenId}
                id={tokenId}
                token={token as Tokens.Table}
                done={done}
                onSourceClick={onSourceClick}
              />
            );

          case "list":
            return (
              <ListToken
                key={tokenId}
                id={tokenId}
                token={token as Tokens.List}
                done={done}
                onSourceClick={onSourceClick}
              />
            );

          case "blockquote":
            return (
              <blockquote
                key={tokenId}
                className="border-l-4 border-muted-foreground/30 pl-4 italic"
              >
                <MarkdownTokens
                  id={`${tokenId}-bq`}
                  tokens={(token as Tokens.Blockquote).tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </blockquote>
            );

          case "hr":
            return <hr key={tokenId} className="my-4 border-border" />;

          case "space":
            return null;

          default:
            // Fallback: render raw text
            return (
              <p key={tokenId} dir="auto">
                {"raw" in token ? String(token.raw) : ""}
              </p>
            );
        }
      })}
    </>
  );
});
```

### 3.3 MarkdownInlineTokens

Renders inline tokens (text, bold, italic, links, etc.).

```typescript
// src/components/query/markdown/MarkdownInlineTokens.tsx
"use client";

import type { Token, Tokens } from "marked";
import { memo } from "react";
import { TextToken } from "./inline/TextToken";
import { CodespanToken } from "./inline/CodespanToken";
import { KatexToken } from "./inline/KatexToken";
import Image from "next/image";

interface MarkdownInlineTokensProps {
  id: string;
  tokens: Token[];
  done?: boolean;
  onSourceClick?: (sourceId: string) => void;
}

export const MarkdownInlineTokens = memo(function MarkdownInlineTokens({
  id,
  tokens,
  done = true,
  onSourceClick,
}: MarkdownInlineTokensProps) {
  return (
    <>
      {tokens.map((token, idx) => {
        const tokenId = `${id}-${idx}`;

        switch (token.type) {
          case "text":
            return <TextToken key={tokenId} token={token} done={done} />;

          case "strong":
            return (
              <strong key={tokenId}>
                <MarkdownInlineTokens
                  id={`${tokenId}-strong`}
                  tokens={(token as Tokens.Strong).tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </strong>
            );

          case "em":
            return (
              <em key={tokenId}>
                <MarkdownInlineTokens
                  id={`${tokenId}-em`}
                  tokens={(token as Tokens.Em).tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </em>
            );

          case "codespan":
            return <CodespanToken key={tokenId} token={token} done={done} />;

          case "link":
            return (
              <a
                key={tokenId}
                href={(token as Tokens.Link).href}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary underline hover:no-underline"
              >
                {(token as Tokens.Link).tokens ? (
                  <MarkdownInlineTokens
                    id={`${tokenId}-link`}
                    tokens={(token as Tokens.Link).tokens}
                    done={done}
                    onSourceClick={onSourceClick}
                  />
                ) : (
                  (token as Tokens.Link).text
                )}
              </a>
            );

          case "image":
            return (
              <img
                key={tokenId}
                src={(token as Tokens.Image).href}
                alt={(token as Tokens.Image).text}
                className="max-w-full h-auto rounded-lg"
              />
            );

          case "br":
            return <br key={tokenId} />;

          case "del":
            return (
              <del key={tokenId}>
                <MarkdownInlineTokens
                  id={`${tokenId}-del`}
                  tokens={(token as Tokens.Del).tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </del>
            );

          case "inlineKatex":
            return (
              <KatexToken
                key={tokenId}
                content={(token as any).text}
                displayMode={false}
              />
            );

          default:
            // Fallback: raw text
            return "text" in token ? String(token.text) : null;
        }
      })}
    </>
  );
});
```

---

## 4. Token Components

### 4.1 TextToken (Streaming-Aware)

Key component for streaming - fades in new text.

```typescript
// src/components/query/markdown/inline/TextToken.tsx
"use client";

import type { Tokens } from "marked";
import { memo } from "react";
import { cn } from "@/lib/utils";

interface TextTokenProps {
  token: Tokens.Text;
  done?: boolean;
}

export const TextToken = memo(function TextToken({
  token,
  done = true,
}: TextTokenProps) {
  return (
    <span
      className={cn(
        "transition-opacity duration-150",
        !done && "animate-fade-in"
      )}
    >
      {token.text}
    </span>
  );
});
```

### 4.2 CodeToken (with Mermaid Support)

```typescript
// src/components/query/markdown/tokens/CodeToken.tsx
"use client";

import type { Tokens } from "marked";
import { memo, useState, useCallback, lazy, Suspense } from "react";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

// Lazy load syntax highlighter
const SyntaxHighlighter = lazy(
  () => import("react-syntax-highlighter/dist/esm/prism-async-light")
);

// Lazy load Mermaid component
const MermaidDiagram = lazy(() => import("../MermaidDiagram"));

interface CodeTokenProps {
  id: string;
  token: Tokens.Code;
  done?: boolean;
  enableMermaid?: boolean;
}

export const CodeToken = memo(function CodeToken({
  id,
  token,
  done = true,
  enableMermaid = true,
}: CodeTokenProps) {
  const [copied, setCopied] = useState(false);
  const language = token.lang || "text";
  const code = token.text;

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    toast.success("Copied to clipboard");
    setTimeout(() => setCopied(false), 2000);
  }, [code]);

  // Mermaid diagrams
  if (language === "mermaid" && enableMermaid) {
    if (!done) {
      // Show placeholder during streaming
      return (
        <div className="my-4 p-6 bg-muted rounded-xl animate-pulse">
          <div className="flex items-center gap-2 text-muted-foreground">
            <span className="text-sm">Rendering diagram...</span>
          </div>
        </div>
      );
    }

    return (
      <Suspense fallback={<div className="my-4 p-6 bg-muted rounded-xl" />}>
        <MermaidDiagram code={code} />
      </Suspense>
    );
  }

  return (
    <div className="relative group my-4 rounded-xl overflow-hidden border border-border">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 bg-muted/50 border-b border-border">
        <span className="text-xs font-medium text-muted-foreground uppercase">
          {language}
        </span>
        <button
          onClick={handleCopy}
          className={cn(
            "p-1.5 rounded-md transition-colors",
            "hover:bg-muted text-muted-foreground hover:text-foreground"
          )}
          aria-label="Copy code"
        >
          {copied ? (
            <Check className="h-4 w-4 text-green-500" />
          ) : (
            <Copy className="h-4 w-4" />
          )}
        </button>
      </div>

      {/* Code content */}
      <Suspense
        fallback={
          <pre className="p-4 overflow-x-auto text-sm">
            <code>{code}</code>
          </pre>
        }
      >
        <SyntaxHighlighter
          language={language}
          className="!m-0 !rounded-none text-sm"
          showLineNumbers={code.split("\n").length > 3}
        >
          {code}
        </SyntaxHighlighter>
      </Suspense>
    </div>
  );
});
```

### 4.3 TableToken

```typescript
// src/components/query/markdown/tokens/TableToken.tsx
"use client";

import type { Tokens } from "marked";
import { memo } from "react";
import { MarkdownInlineTokens } from "../MarkdownInlineTokens";
import { cn } from "@/lib/utils";

interface TableTokenProps {
  id: string;
  token: Tokens.Table;
  done?: boolean;
  onSourceClick?: (sourceId: string) => void;
}

export const TableToken = memo(function TableToken({
  id,
  token,
  done = true,
  onSourceClick,
}: TableTokenProps) {
  return (
    <div className="my-4 overflow-x-auto rounded-lg border border-border">
      <table className="w-full text-sm">
        <thead className="bg-muted/50">
          <tr>
            {token.header.map((cell, idx) => (
              <th
                key={`${id}-h-${idx}`}
                className={cn(
                  "px-4 py-2 text-left font-semibold",
                  token.align?.[idx] && `text-${token.align[idx]}`
                )}
              >
                <MarkdownInlineTokens
                  id={`${id}-header-${idx}`}
                  tokens={cell.tokens}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {token.rows.map((row, rowIdx) => (
            <tr
              key={`${id}-r-${rowIdx}`}
              className="border-t border-border hover:bg-muted/30"
            >
              {row.map((cell, cellIdx) => (
                <td
                  key={`${id}-c-${rowIdx}-${cellIdx}`}
                  className={cn(
                    "px-4 py-2",
                    token.align?.[cellIdx] && `text-${token.align[cellIdx]}`
                  )}
                >
                  <MarkdownInlineTokens
                    id={`${id}-cell-${rowIdx}-${cellIdx}`}
                    tokens={cell.tokens}
                    done={done}
                    onSourceClick={onSourceClick}
                  />
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
});
```

---

## 5. Marked Extensions

### 5.1 Configure Marked

```typescript
// src/components/query/markdown/utils/configure-marked.ts
import { marked } from "marked";
import { markedKatexExtension } from "./katex-extension";
import { citationExtension } from "./citation-extension";

let configured = false;

export function configureMarked() {
  if (configured) return;

  marked.use({
    breaks: true,
    gfm: true,
  });

  // KaTeX extension for math
  marked.use(markedKatexExtension({ throwOnError: false }));

  // Citation extension for [1], [2] etc.
  marked.use(citationExtension());

  configured = true;
}
```

### 5.2 KaTeX Extension

```typescript
// src/components/query/markdown/utils/katex-extension.ts
import type { MarkedExtension } from "marked";

interface KatexOptions {
  throwOnError?: boolean;
}

export function markedKatexExtension(
  options: KatexOptions = {}
): MarkedExtension {
  return {
    extensions: [
      {
        name: "inlineKatex",
        level: "inline",
        start(src: string) {
          return src.indexOf("$");
        },
        tokenizer(src: string) {
          // Match $...$ for inline math
          const match = src.match(/^\$([^$\n]+)\$/);
          if (match) {
            return {
              type: "inlineKatex",
              raw: match[0],
              text: match[1].trim(),
            };
          }
          return undefined;
        },
      },
      {
        name: "blockKatex",
        level: "block",
        start(src: string) {
          return src.indexOf("$$");
        },
        tokenizer(src: string) {
          // Match $$...$$ for block math
          const match = src.match(/^\$\$([^$]+)\$\$/);
          if (match) {
            return {
              type: "blockKatex",
              raw: match[0],
              text: match[1].trim(),
            };
          }
          return undefined;
        },
      },
    ],
  };
}
```

---

## 6. Migration Steps

### 6.1 Phase 1: Install Dependencies

```bash
cd edgequake_webui
bun add marked @types/marked
```

### 6.2 Phase 2: Create Components

1. Create directory structure as shown in Section 2.1
2. Implement components in order:
   - `utils/configure-marked.ts`
   - `inline/TextToken.tsx`
   - `MarkdownInlineTokens.tsx`
   - `tokens/CodeToken.tsx`
   - `MarkdownTokens.tsx`
   - `StreamingMarkdownRenderer.tsx`

### 6.3 Phase 3: Migrate Usage

Replace in `chat-message.tsx`:

```diff
- import { MarkdownRenderer } from './markdown-renderer';
+ import { StreamingMarkdownRenderer } from './markdown/StreamingMarkdownRenderer';

// In component:
- <MarkdownRenderer content={content} isStreaming={isStreaming} />
+ <StreamingMarkdownRenderer content={content} isStreaming={isStreaming} />
```

### 6.4 Phase 4: Remove Old Code

After validation, remove:

- `markdown-renderer.tsx`
- `markdown-renderer-old.tsx`

---

## 7. Testing Checklist

| Test                      | Expected Result                    |
| ------------------------- | ---------------------------------- |
| Streaming paragraph       | Text fades in progressively        |
| Bold/italic during stream | Renders correctly, no fallback     |
| Code block during stream  | Shows placeholder, then code       |
| Mermaid during stream     | Shows "Rendering...", then diagram |
| Table during stream       | Renders progressively              |
| KaTeX math                | Renders equation correctly         |
| Copy code button          | Copies code, shows toast           |
| Error boundary            | Fallback renders on error          |

---

_Last updated: 2024-12-27_
