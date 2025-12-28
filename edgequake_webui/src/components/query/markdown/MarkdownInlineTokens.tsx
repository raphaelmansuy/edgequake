/**
 * Inline Token Components
 * 
 * Renders inline markdown tokens (text, bold, italic, code, links, etc.)
 */
'use client';

import { cn } from '@/lib/utils';
import type { Token, Tokens } from 'marked';
import { lazy, memo, Suspense } from 'react';

// Lazy load KaTeX for performance
const KatexMath = lazy(() => import('./KatexMath'));

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
          case 'text': {
            const textToken = token as Tokens.Text;
            // During streaming, add a subtle fade effect to the last text
            const isLastToken = idx === tokens.length - 1 && !done;
            return (
              <span
                key={tokenId}
                className={cn(isLastToken && 'animate-pulse')}
              >
                {/* Handle nested tokens in text (like bold inside text) */}
                {textToken.tokens ? (
                  <MarkdownInlineTokens
                    id={tokenId}
                    tokens={textToken.tokens}
                    done={done}
                    onSourceClick={onSourceClick}
                  />
                ) : (
                  textToken.text
                )}
              </span>
            );
          }

          case 'strong': {
            const strongToken = token as Tokens.Strong;
            return (
              <strong key={tokenId} className="font-semibold">
                <MarkdownInlineTokens
                  id={tokenId}
                  tokens={strongToken.tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </strong>
            );
          }

          case 'em': {
            const emToken = token as Tokens.Em;
            return (
              <em key={tokenId}>
                <MarkdownInlineTokens
                  id={tokenId}
                  tokens={emToken.tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </em>
            );
          }

          case 'del': {
            const delToken = token as Tokens.Del;
            return (
              <del key={tokenId} className="line-through text-muted-foreground">
                <MarkdownInlineTokens
                  id={tokenId}
                  tokens={delToken.tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </del>
            );
          }

          case 'codespan': {
            const codeToken = token as Tokens.Codespan;
            return (
              <code
                key={tokenId}
                className="rounded bg-muted px-1.5 py-0.5 font-mono text-sm text-foreground"
              >
                {codeToken.text}
              </code>
            );
          }

          case 'link': {
            const linkToken = token as Tokens.Link;
            return (
              <a
                key={tokenId}
                href={linkToken.href}
                title={linkToken.title ?? undefined}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary underline underline-offset-2 hover:text-primary/80 transition-colors"
              >
                <MarkdownInlineTokens
                  id={tokenId}
                  tokens={linkToken.tokens || []}
                  done={done}
                  onSourceClick={onSourceClick}
                />
              </a>
            );
          }

          case 'image': {
            const imgToken = token as Tokens.Image;
            // Skip rendering if href is empty or undefined to avoid browser warnings
            if (!imgToken.href) {
              return (
                <span key={tokenId} className="text-muted-foreground italic">
                  [Image: {imgToken.text || 'no alt text'}]
                </span>
              );
            }
            return (
              <img
                key={tokenId}
                src={imgToken.href}
                alt={imgToken.text}
                title={imgToken.title ?? undefined}
                className="max-w-full rounded-lg my-2"
                loading="lazy"
              />
            );
          }

          case 'br':
            return <br key={tokenId} />;

          // Custom math inline extension
          case 'math_inline': {
            const mathToken = token as unknown as { text: string };
            return (
              <Suspense
                key={tokenId}
                fallback={
                  <code className="rounded bg-muted px-1 py-0.5 font-mono text-sm">
                    {mathToken.text}
                  </code>
                }
              >
                <KatexMath math={mathToken.text} block={false} />
              </Suspense>
            );
          }

          // Custom citation extension
          case 'citation': {
            const citationToken = token as unknown as { sourceId: string };
            return (
              <button
                key={tokenId}
                onClick={() => onSourceClick?.(citationToken.sourceId)}
                className="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs font-medium text-primary bg-primary/10 rounded-md hover:bg-primary/20 transition-colors"
              >
                <span className="text-[10px]">📄</span>
                <span>{citationToken.sourceId}</span>
              </button>
            );
          }

          // Escape HTML entities
          case 'escape': {
            const escapeToken = token as Tokens.Escape;
            return <span key={tokenId}>{escapeToken.text}</span>;
          }

          // HTML tokens (sanitized)
          case 'html': {
            const htmlToken = token as Tokens.HTML;
            // Only render safe inline HTML like <br>, <wbr>
            if (/^<(br|wbr)\s*\/?>/i.test(htmlToken.raw)) {
              return <br key={tokenId} />;
            }
            // Otherwise render as text
            return <span key={tokenId}>{htmlToken.text}</span>;
          }

          default:
            // Unknown token - render raw text if available
            if ('text' in token && typeof token.text === 'string') {
              return <span key={tokenId}>{token.text}</span>;
            }
            return null;
        }
      })}
    </>
  );
});

export default MarkdownInlineTokens;
