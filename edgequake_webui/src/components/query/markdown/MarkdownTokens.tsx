/**
 * Markdown Block Token Renderer
 * 
 * Renders block-level markdown tokens (heading, paragraph, code, table, list, etc.)
 * This is the main token renderer that handles the document structure.
 * 
 * Key features:
 * - GitHub-style alerts (NOTE, TIP, WARNING, CAUTION, IMPORTANT)
 * - Collapsible details blocks
 * - Table streaming with skeleton loading
 * - Lazy-loaded heavy components
 */
'use client';

import { cn } from '@/lib/utils';
import type { Token, Tokens } from 'marked';
import { lazy, memo, Suspense, useId } from 'react';
import { MarkdownInlineTokens } from './MarkdownInlineTokens';
import type { AlertType } from './utils/configure-marked';
import { sanitizeHtml } from './utils/sanitize-html';

// Lazy load heavy components
const CodeBlock = lazy(() => import('./CodeBlock'));
const MermaidBlock = lazy(() => import('./MermaidBlock'));
const KatexMath = lazy(() => import('./KatexMath'));
const GitHubAlert = lazy(() => import('./GitHubAlert'));
const DetailsBlock = lazy(() => import('./DetailsBlock'));
const TableSkeleton = lazy(() => import('./TableSkeleton'));

interface MarkdownTokensProps {
  tokens: Token[];
  isStreaming?: boolean;
  className?: string;
  onSourceClick?: (sourceId: string) => void;
}

interface TokenRendererProps {
  token: Token;
  tokenId: string;
  isStreaming?: boolean;
  isLastToken?: boolean;
  onSourceClick?: (sourceId: string) => void;
}

// Skeleton loader for code blocks
function CodeBlockSkeleton() {
  return (
    <div className="my-4 animate-pulse rounded-lg border border-border bg-muted/50">
      <div className="border-b border-border bg-muted px-4 py-2">
        <div className="h-3 w-20 rounded bg-muted-foreground/20" />
      </div>
      <div className="p-4 space-y-2">
        <div className="h-3 w-3/4 rounded bg-muted-foreground/20" />
        <div className="h-3 w-1/2 rounded bg-muted-foreground/20" />
        <div className="h-3 w-2/3 rounded bg-muted-foreground/20" />
      </div>
    </div>
  );
}

// Skeleton loader for math blocks
function MathBlockSkeleton() {
  return (
    <div className="my-4 flex justify-center animate-pulse">
      <div className="h-12 w-48 rounded bg-muted-foreground/20" />
    </div>
  );
}

// Skeleton loader for alerts
function AlertSkeleton() {
  return (
    <div className="my-4 animate-pulse rounded-lg border-l-4 border-border bg-muted/50 p-4">
      <div className="flex items-start gap-3">
        <div className="h-5 w-5 rounded bg-muted-foreground/20" />
        <div className="flex-1 space-y-2">
          <div className="h-4 w-20 rounded bg-muted-foreground/20" />
          <div className="h-3 w-full rounded bg-muted-foreground/20" />
          <div className="h-3 w-3/4 rounded bg-muted-foreground/20" />
        </div>
      </div>
    </div>
  );
}

/**
 * Renders a single block-level token
 */
const TokenRenderer = memo(function TokenRenderer({
  token,
  tokenId,
  isStreaming = false,
  isLastToken = false,
  onSourceClick,
}: TokenRendererProps) {
  switch (token.type) {
    case 'heading': {
      const heading = token as Tokens.Heading;
      const Tag = `h${heading.depth}` as 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6';
      const headingStyles: Record<number, string> = {
        1: 'text-2xl font-bold mt-6 mb-4',
        2: 'text-xl font-semibold mt-5 mb-3',
        3: 'text-lg font-medium mt-4 mb-2',
        4: 'text-base font-medium mt-3 mb-2',
        5: 'text-sm font-medium mt-2 mb-1',
        6: 'text-sm font-medium mt-2 mb-1 text-muted-foreground',
      };
      return (
        <Tag className={headingStyles[heading.depth]}>
          <MarkdownInlineTokens
            id={`${tokenId}-heading`}
            tokens={heading.tokens}
            done={!isStreaming || !isLastToken}
            onSourceClick={onSourceClick}
          />
        </Tag>
      );
    }

    case 'paragraph': {
      const paragraph = token as Tokens.Paragraph;
      return (
        <p className="my-3 leading-7">
          <MarkdownInlineTokens
            id={`${tokenId}-para`}
            tokens={paragraph.tokens}
            done={!isStreaming || !isLastToken}
            onSourceClick={onSourceClick}
          />
        </p>
      );
    }

    case 'code': {
      const code = token as Tokens.Code;
      
      // Handle Mermaid diagrams
      if (code.lang?.toLowerCase() === 'mermaid') {
        return (
          <Suspense fallback={<CodeBlockSkeleton />}>
            <MermaidBlock
              code={code.text}
              isStreaming={isStreaming && isLastToken}
            />
          </Suspense>
        );
      }
      
      // Regular code blocks
      return (
        <Suspense fallback={<CodeBlockSkeleton />}>
          <CodeBlock code={code.text} language={code.lang} />
        </Suspense>
      );
    }

    case 'math_block': {
      // Custom token type from our marked extension
      const mathToken = token as Token & { raw: string };
      // Extract math content - remove $$ delimiters
      const mathContent = mathToken.raw
        .replace(/^\$\$\s*/, '')
        .replace(/\s*\$\$$/, '')
        .trim();
      return (
        <Suspense fallback={<MathBlockSkeleton />}>
          <div className="my-4 overflow-x-auto">
            <KatexMath math={mathContent} block />
          </div>
        </Suspense>
      );
    }

    case 'blockquote': {
      const blockquote = token as Tokens.Blockquote;
      return (
        <blockquote className="my-4 border-l-4 border-primary/50 pl-4 italic text-muted-foreground">
          <MarkdownTokens
            tokens={blockquote.tokens}
            isStreaming={isStreaming}
            onSourceClick={onSourceClick}
          />
        </blockquote>
      );
    }

    case 'list': {
      const list = token as Tokens.List;
      const Tag = list.ordered ? 'ol' : 'ul';
      const listStyle = list.ordered
        ? 'list-decimal pl-6 my-3 space-y-1'
        : 'list-disc pl-6 my-3 space-y-1';
      
      return (
        <Tag className={listStyle} start={list.start || undefined}>
          {list.items.map((item, index) => (
            <li
              key={index}
              className={cn(
                item.loose ? 'leading-loose' : 'leading-7',
                // Task items get special flex layout and no bullet
                item.task && 'flex items-start gap-2 list-none -ml-6'
              )}
            >
              {/* Only render checkbox for ACTUAL task items (item.task === true) */}
              {item.task && (
                <input
                  type="checkbox"
                  checked={item.checked ?? false}
                  disabled
                  aria-label={item.checked ? 'Completed task' : 'Incomplete task'}
                  className="mt-1.5 h-4 w-4 shrink-0 rounded border-border text-primary accent-primary"
                />
              )}
              <div className={item.task ? 'flex-1 min-w-0' : undefined}>
                <MarkdownTokens
                  tokens={item.tokens}
                  isStreaming={isStreaming && index === list.items.length - 1}
                  onSourceClick={onSourceClick}
                />
              </div>
            </li>
          ))}
        </Tag>
      );
    }

    case 'table': {
      const table = token as Tokens.Table;
      return (
        <div className="my-4 overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50">
                {table.header.map((cell, index) => (
                  <th
                    key={index}
                    className={cn(
                      'px-4 py-2 text-left font-medium text-foreground',
                      table.align[index] === 'center' && 'text-center',
                      table.align[index] === 'right' && 'text-right'
                    )}
                  >
                    <MarkdownInlineTokens
                      id={`${tokenId}-th-${index}`}
                      tokens={cell.tokens}
                      done
                      onSourceClick={onSourceClick}
                    />
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {table.rows.map((row, rowIndex) => (
                <tr
                  key={rowIndex}
                  className="border-b border-border hover:bg-muted/30"
                >
                  {row.map((cell, cellIndex) => (
                    <td
                      key={cellIndex}
                      className={cn(
                        'px-4 py-2 text-foreground/90',
                        table.align[cellIndex] === 'center' && 'text-center',
                        table.align[cellIndex] === 'right' && 'text-right'
                      )}
                    >
                      <MarkdownInlineTokens
                        id={`${tokenId}-td-${rowIndex}-${cellIndex}`}
                        tokens={cell.tokens}
                        done
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
    }

    case 'hr':
      // Subtle divider that doesn't look like an artifact
      return (
        <hr className="my-6 border-0 h-px bg-gradient-to-r from-transparent via-border to-transparent" />
      );

    case 'space':
      return null;

    case 'html': {
      const html = token as Tokens.HTML;
      // Sanitize HTML using DOMPurify for security
      const sanitizedHtml = sanitizeHtml(html.raw);
      return (
        <div
          className="my-3"
          dangerouslySetInnerHTML={{ __html: sanitizedHtml }}
        />
      );
    }

    // GitHub-style alert blocks
    case 'github_alert': {
      const alertToken = token as unknown as { alertType: AlertType; text: string };
      return (
        <Suspense fallback={<AlertSkeleton />}>
          <GitHubAlert type={alertToken.alertType}>
            {alertToken.text}
          </GitHubAlert>
        </Suspense>
      );
    }

    // Collapsible details blocks
    case 'details': {
      const detailsToken = token as unknown as { summary: string; content: string; open: boolean };
      return (
        <Suspense fallback={<CodeBlockSkeleton />}>
          <DetailsBlock 
            summary={detailsToken.summary} 
            defaultOpen={detailsToken.open}
          >
            {detailsToken.content}
          </DetailsBlock>
        </Suspense>
      );
    }

    case 'text': {
      const text = token as Tokens.Text;
      // Text tokens at block level might have nested tokens
      if ('tokens' in text && text.tokens) {
        return (
          <MarkdownInlineTokens
            id={`${tokenId}-text`}
            tokens={text.tokens}
            done={!isStreaming || !isLastToken}
            onSourceClick={onSourceClick}
          />
        );
      }
      return <span>{text.raw}</span>;
    }

    default:
      // Log unknown tokens for debugging
      if (process.env.NODE_ENV === 'development') {
        console.warn('Unknown token type:', token.type, token);
      }
      // Fallback: render raw text if available
      if ('raw' in token && typeof token.raw === 'string') {
        return <span>{token.raw}</span>;
      }
      return null;
  }
});

/**
 * Main component that renders an array of block tokens
 */
export const MarkdownTokens = memo(function MarkdownTokens({
  tokens,
  isStreaming = false,
  className,
  onSourceClick,
}: MarkdownTokensProps) {
  const baseId = useId();
  
  return (
    <div className={cn('markdown-content', className)}>
      {tokens.map((token, index) => (
        <TokenRenderer
          key={index}
          token={token}
          tokenId={`${baseId}-${index}`}
          isStreaming={isStreaming}
          isLastToken={index === tokens.length - 1}
          onSourceClick={onSourceClick}
        />
      ))}
    </div>
  );
});

export default MarkdownTokens;
