'use client';

import { cn } from '@/lib/utils';
import { Check, ChevronDown, ChevronUp, Copy, FileCode2 } from 'lucide-react';
import { memo, useCallback, useMemo, useState, type ReactNode } from 'react';

interface CodeBlockProps {
  code: string;
  language: string;
  showLineNumbers?: boolean;
  collapsible?: boolean;
  maxLines?: number;
  filename?: string;
  highlightLines?: number[];
  className?: string;
  children?: ReactNode;
}

/**
 * Enhanced CodeBlock component with:
 * - Line numbers
 * - Copy to clipboard with feedback
 * - Collapsible long code
 * - Language badge
 * - Syntax highlighting (via rehype-highlight)
 */
export const CodeBlock = memo(function CodeBlock({
  code,
  language,
  showLineNumbers = true,
  collapsible = true,
  maxLines = 20,
  filename,
  highlightLines = [],
  className,
  children,
}: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const [expanded, setExpanded] = useState(false);

  // Parse lines from code for line numbers
  const lines = useMemo(() => code.split('\n'), [code]);
  const lineCount = lines.length;
  const shouldCollapse = collapsible && lineCount > maxLines && !expanded;

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }, [code]);

  const toggleExpand = useCallback(() => {
    setExpanded(prev => !prev);
  }, []);

  // Language display name (capitalize first letter)
  const languageDisplay = language 
    ? language.charAt(0).toUpperCase() + language.slice(1).toLowerCase()
    : 'Code';

  return (
    <div 
      className={cn(
        'code-block-container group my-4 rounded-xl overflow-hidden',
        'bg-[oklch(0.13_0.01_265)]',
        'border border-[oklch(0.25_0.01_265)]',
        'shadow-sm',
        className
      )}
    >
      {/* Header Bar */}
      <div 
        className={cn(
          'code-block-header flex items-center justify-between',
          'px-4 py-2.5',
          'bg-[oklch(0.18_0.01_265)]',
          'border-b border-[oklch(0.25_0.01_265)]'
        )}
      >
        {/* Left: Language badge or filename */}
        <div className="flex items-center gap-2">
          <FileCode2 className="h-3.5 w-3.5 text-[oklch(0.55_0.01_265)]" />
          {filename ? (
            <span className="text-xs font-medium text-[oklch(0.75_0.01_265)] font-mono">
              {filename}
            </span>
          ) : (
            <span className="text-xs font-medium text-[oklch(0.65_0.01_265)] uppercase tracking-wide">
              {languageDisplay}
            </span>
          )}
        </div>

        {/* Right: Line count + Copy button */}
        <div className="flex items-center gap-3">
          {lineCount > 1 && (
            <span className="text-xs text-[oklch(0.5_0.01_265)]">
              {lineCount} lines
            </span>
          )}
          <button
            onClick={handleCopy}
            className={cn(
              'flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs',
              'transition-all duration-200',
              'bg-[oklch(0.22_0.01_265)] hover:bg-[oklch(0.3_0.01_265)]',
              'text-[oklch(0.65_0.01_265)] hover:text-[oklch(0.85_0.01_265)]',
              copied && 'bg-green-500/20 text-green-400'
            )}
            title={copied ? 'Copied!' : 'Copy code'}
            aria-label={copied ? 'Copied!' : 'Copy code'}
          >
            {copied ? (
              <>
                <Check className="h-3.5 w-3.5" />
                <span className="hidden sm:inline">Copied!</span>
              </>
            ) : (
              <>
                <Copy className="h-3.5 w-3.5" />
                <span className="hidden sm:inline">Copy</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Code Content */}
      <div className="relative">
        {/* Line Numbers Gutter */}
        {showLineNumbers && (
          <div 
            className={cn(
              'absolute left-0 top-0 bottom-0 w-12',
              'py-4 pr-2',
              'text-right select-none',
              'text-[oklch(0.4_0.01_265)]',
              'text-xs leading-6 font-mono',
              'bg-[oklch(0.1_0.01_265)]',
              'border-r border-[oklch(0.2_0.01_265)]'
            )}
            aria-hidden="true"
          >
            {lines.slice(0, shouldCollapse ? maxLines : undefined).map((_, index) => {
              const lineNumber = index + 1;
              const isHighlighted = highlightLines.includes(lineNumber);
              return (
                <div 
                  key={lineNumber}
                  className={cn(
                    'px-2 h-6 leading-6',
                    isHighlighted && 'bg-primary/10 text-primary'
                  )}
                >
                  {lineNumber}
                </div>
              );
            })}
            {shouldCollapse && (
              <div className="px-2 h-6 leading-6 text-[oklch(0.3_0.01_265)]">...</div>
            )}
          </div>
        )}

        {/* Code Content Area */}
        <div 
          className={cn(
            'overflow-x-auto',
            'py-4',
            showLineNumbers ? 'pl-14 pr-4' : 'px-4',
            'text-sm leading-6 font-mono',
            'text-[oklch(0.88_0.02_265)]',
            shouldCollapse && 'max-h-[calc(20*1.5rem+2rem)] overflow-hidden'
          )}
        >
          {children ? (
            // Use provided children (pre-highlighted from rehype-highlight)
            // Ensure proper styling overrides
            <div className="overflow-x-auto [&_pre]:!bg-transparent [&_pre]:!p-0 [&_pre]:!m-0 [&_code]:!bg-transparent">
              {children}
            </div>
          ) : (
            // Fallback: render code directly without highlighting
            <pre className="!bg-transparent !p-0 !m-0 whitespace-pre">
              <code className={`language-${language}`}>
                {shouldCollapse ? lines.slice(0, maxLines).join('\n') : code}
              </code>
            </pre>
          )}
        </div>

        {/* Collapse/Expand Gradient & Button */}
        {collapsible && lineCount > maxLines && (
          <>
            {shouldCollapse && (
              <div 
                className={cn(
                  'absolute bottom-0 left-0 right-0 h-20',
                  'bg-gradient-to-t from-[oklch(0.13_0.01_265)] via-[oklch(0.13_0.01_265)]/80 to-transparent',
                  'pointer-events-none'
                )}
              />
            )}
            <button
              onClick={toggleExpand}
              className={cn(
                'w-full py-2.5 px-4',
                'bg-[oklch(0.16_0.01_265)]',
                'border-t border-[oklch(0.25_0.01_265)]',
                'text-xs text-[oklch(0.55_0.01_265)] hover:text-[oklch(0.75_0.01_265)]',
                'hover:bg-[oklch(0.2_0.01_265)]',
                'transition-colors duration-200',
                'flex items-center justify-center gap-2',
                'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary'
              )}
              aria-expanded={expanded}
            >
              {expanded ? (
                <>
                  <ChevronUp className="h-4 w-4" />
                  <span>Collapse</span>
                </>
              ) : (
                <>
                  <ChevronDown className="h-4 w-4" />
                  <span>Show all {lineCount} lines</span>
                </>
              )}
            </button>
          </>
        )}
      </div>
    </div>
  );
});

export default CodeBlock;
