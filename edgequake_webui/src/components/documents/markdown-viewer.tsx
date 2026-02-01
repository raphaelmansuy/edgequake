/**
 * @module MarkdownViewer
 * @description Reusable Markdown viewer component with copy and scroll features.
 * Displays extracted markdown content with syntax highlighting and formatting.
 *
 * @implements SPEC-002 - Document Viewer with Markdown display
 * @implements FEAT0721 - Markdown rendering with syntax highlighting
 * @implements FEAT0722 - Copy content to clipboard
 * @implements FEAT0723 - Line numbers display
 *
 * @enforces BR0721 - Smooth scrolling within container
 * @enforces BR0722 - Proper typography and spacing
 *
 * @see {@link docs/features.md} FEAT0721-0723
 */
'use client';

import { StreamingMarkdownRenderer } from '@/components/query/markdown';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { Check, Copy, FileText } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface MarkdownViewerProps {
  /** Markdown content to display */
  content: string | null;
  /** Optional class name for container */
  className?: string;
  /** Fixed height for container (enables scrolling) */
  height?: number;
  /** Whether to show the toolbar */
  showToolbar?: boolean;
  /** Whether to show line numbers */
  showLineNumbers?: boolean;
  /** Title displayed in toolbar */
  title?: string;
}

/**
 * MarkdownViewer component for displaying markdown content.
 *
 * Uses the existing StreamingMarkdownRenderer for high-quality markdown rendering
 * with syntax highlighting, code blocks, tables, and math support.
 */
export function MarkdownViewer({
  content,
  className,
  height,
  showToolbar = true,
  showLineNumbers = false,
  title = 'Extracted Markdown',
}: MarkdownViewerProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      toast.success(t('common.copied', 'Copied to clipboard'));
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [content, t]);

  if (!content) {
    return (
      <div className="flex flex-col items-center justify-center p-8 text-center text-muted-foreground">
        <FileText className="h-12 w-12 mb-4 opacity-50" />
        <p>{t('documents.viewer.noMarkdown', 'No markdown content available')}</p>
        <p className="text-sm mt-2">
          {t('documents.viewer.noMarkdownHint', 'The document may not have been processed yet.')}
        </p>
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col', className)}>
      {/* Toolbar */}
      {showToolbar && (
        <div className="flex items-center justify-between gap-2 p-2 border-b bg-muted/30">
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">{title}</span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="h-8"
            onClick={handleCopy}
            title={t('common.copy', 'Copy to clipboard')}
          >
            {copied ? (
              <Check className="h-4 w-4 text-green-500" />
            ) : (
              <Copy className="h-4 w-4" />
            )}
            <span className="ml-1.5 hidden sm:inline">
              {copied ? t('common.copied', 'Copied') : t('common.copy', 'Copy')}
            </span>
          </Button>
        </div>
      )}

      {/* Markdown Content */}
      <div
        className={cn(
          'flex-1 overflow-auto bg-background',
          'scroll-smooth'
        )}
        style={{ height: height ? `${height}px` : 'auto' }}
      >
        <div className={cn(
          'p-4 md:p-6',
          'prose prose-sm md:prose-base dark:prose-invert max-w-none',
          // Typography improvements for document viewing
          'prose-headings:scroll-mt-4',
          'prose-pre:bg-muted/50 prose-pre:border prose-pre:border-border',
          'prose-code:before:content-none prose-code:after:content-none',
          'prose-table:text-sm',
          showLineNumbers && 'markdown-with-line-numbers'
        )}>
          <StreamingMarkdownRenderer
            content={content}
            isStreaming={false}
          />
        </div>
      </div>
    </div>
  );
}

export default MarkdownViewer;
