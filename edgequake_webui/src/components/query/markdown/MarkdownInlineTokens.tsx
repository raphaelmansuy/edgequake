/**
 * Inline Token Components
 * 
 * Renders inline markdown tokens (text, bold, italic, code, links, etc.)
 */
'use client';

import { cn } from '@/lib/utils';
import type { Token, Tokens } from 'marked';
import { memo } from 'react';
import { MathTokenRenderer } from './MathTokenRenderer';
import { sanitizeHtml } from './utils/sanitize-html';

/**
 * Merge split HTML tag tokens back into whole elements.
 *
 * WHY: marked.js tokenizes `<sup>1*</sup>` as THREE separate tokens:
 *   [html("<sup>"), text("1*"), html("</sup>")]
 * Rendering them individually means DOMPurify strips the empty `<sup>`, the
 * text "1*" renders unstyled, and `</sup>` appears as literal text.
 *
 * This function reassembles matching open/close pairs (non-nested) into one
 * html token so DOMPurify can sanitize and render the whole element.
 *
 * First Principles: scan → identify opening tag → find closing tag → merge raw.
 */
function mergeInlineHtmlTokens(tokens: Token[]): Token[] {
  const merged: Token[] = [];
  let i = 0;

  while (i < tokens.length) {
    const tok = tokens[i];

    // Only attempt merge when we find an opening (non-self-closing, non-closing) HTML tag
    const openTagMatch =
      tok.type === 'html' &&
      !tok.raw.startsWith('</') &&
      !tok.raw.endsWith('/>') &&
      /^<([a-z][a-z0-9]*)\b/i.exec(tok.raw);

    if (openTagMatch) {
      const tag = openTagMatch[1].toLowerCase();
      const closingRE = new RegExp(`^</${tag}\\s*>$`, 'i');

      // Scan ahead for the matching closing tag (simple linear scan, no nesting)
      let found = false;
      for (let j = i + 1; j < tokens.length; j++) {
        if (tokens[j].type === 'html' && closingRE.test(tokens[j].raw.trim())) {
          // Reconstruct the full element from raw text of intermediate tokens
          const inner = tokens.slice(i + 1, j).map(t => t.raw).join('');
          const fullHtml = tok.raw + inner + tokens[j].raw;
          merged.push({
            type: 'html' as const,
            raw: fullHtml,
            text: fullHtml,
            inLink: false,
            inRawBlock: false,
            block: false,
          } as Token);
          i = j + 1;
          found = true;
          break;
        }
      }

      if (!found) {
        // No matching close tag — keep the opening tag as-is (will be stripped by DOMPurify)
        merged.push(tok);
        i++;
      }
      continue;
    }

    merged.push(tok);
    i++;
  }

  return merged;
}

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
  // Merge split HTML tag tokens (e.g. <sup>, <sub>) before rendering
  const normalizedTokens = mergeInlineHtmlTokens(tokens);

  return (
    <>
      {normalizedTokens.map((token, idx) => {
        const tokenId = `${id}-${idx}`;

        switch (token.type) {
          case 'text': {
            const textToken = token as Tokens.Text;
            // During streaming, add a subtle fade effect to the last text
            const isLastToken = idx === tokens.length - 1 && !done;
            return (
              <span
                key={tokenId}
                className={cn(isLastToken && 'motion-safe:animate-pulse')}
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
                className="text-primary underline underline-offset-2 hover:text-primary/80 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 rounded-sm"
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
              // eslint-disable-next-line @next/next/no-img-element
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

          case 'math_inline':
          case 'math_paren_inline':
            return <MathTokenRenderer key={tokenId} token={token} />;

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

          // HTML tokens — sanitized via DOMPurify.
          // Merged html elements (e.g. <sup>1*</sup>) are sanitized and injected.
          // Standalone orphan closing tags (e.g. </sup>) are suppressed — they
          // can only appear here when mergeInlineHtmlTokens found no opening pair.
          case 'html': {
            const htmlToken = token as Tokens.HTML;
            // Suppress orphan closing tags — they have no meaningful content
            if (/^<\/[a-z][a-z0-9]*\s*>/i.test(htmlToken.raw)) {
              return null;
            }
            const sanitized = sanitizeHtml(htmlToken.raw);
            if (!sanitized) return null;
            return (
              <span
                key={tokenId}
                // eslint-disable-next-line react/no-danger
                dangerouslySetInnerHTML={{ __html: sanitized }}
              />
            );
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
