/**
 * @module PDFMarkdownSplitView
 * @description Side-by-side view of PDF and extracted Markdown.
 * Supports three modes: PDF only, Markdown only, or Split view.
 *
 * @implements SPEC-002 - Document Viewer with PDF+Markdown display
 * @implements FEAT0731 - PDF and Markdown side-by-side view
 * @implements FEAT0732 - View mode toggle
 * @implements FEAT0733 - Responsive layout
 *
 * @enforces BR0731 - Smooth scrolling in both panels
 * @enforces BR0732 - Clear visual separation between panels
 *
 * @see {@link docs/features.md} FEAT0731-0733
 */
'use client';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { Columns, FileText, FileType } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MarkdownViewer } from './markdown-viewer';
import { PDFViewer } from './pdf-viewer';

/** View mode for the split view component */
type ViewMode = 'pdf' | 'markdown' | 'split';

interface PDFMarkdownSplitViewProps {
  /** URL to the PDF file */
  pdfUrl: string;
  /** Extracted markdown content */
  markdown: string | null;
  /** Optional class name for container */
  className?: string;
  /** Fixed height for the container */
  height?: number;
  /** Initial view mode (defaults to 'split') */
  initialMode?: ViewMode;
}

/**
 * PDFMarkdownSplitView - Display PDF and Markdown side by side.
 *
 * WHY-OODA82: For PDF-origin documents, users need to see both the original
 * PDF and the extracted markdown to verify extraction quality and compare content.
 *
 * Features:
 * - Three view modes: PDF only, Markdown only, Split view
 * - Responsive layout (stacked on mobile, side-by-side on desktop)
 * - Independent scrolling for each panel
 * - Clear visual separation between panels
 */
export function PDFMarkdownSplitView({
  pdfUrl,
  markdown,
  className,
  height = 500,
  initialMode = 'split',
}: PDFMarkdownSplitViewProps) {
  const { t } = useTranslation();
  const [viewMode, setViewMode] = useState<ViewMode>(initialMode);

  const handleModeChange = useCallback((mode: ViewMode) => {
    setViewMode(mode);
  }, []);

  // Determine panel visibility based on mode
  const showPdf = viewMode === 'pdf' || viewMode === 'split';
  const showMarkdown = viewMode === 'markdown' || viewMode === 'split';

  return (
    <div className={cn('flex flex-col', className)}>
      {/* Toolbar with view mode toggles */}
      <div className="flex items-center justify-between gap-2 p-2 border-b bg-muted/30">
        <div className="flex items-center gap-1">
          <span className="text-sm font-medium text-muted-foreground mr-2">
            {t('documents.viewer.viewMode', 'View:')}
          </span>
          
          {/* PDF Only Button */}
          <Button
            variant={viewMode === 'pdf' ? 'secondary' : 'ghost'}
            size="sm"
            className="h-8"
            onClick={() => handleModeChange('pdf')}
            title={t('documents.viewer.pdfOnly', 'PDF Only')}
          >
            <FileType className="h-4 w-4 mr-1.5" />
            <span className="hidden sm:inline">PDF</span>
          </Button>

          {/* Split View Button */}
          <Button
            variant={viewMode === 'split' ? 'secondary' : 'ghost'}
            size="sm"
            className="h-8"
            onClick={() => handleModeChange('split')}
            title={t('documents.viewer.splitView', 'Side by Side')}
          >
            <Columns className="h-4 w-4 mr-1.5" />
            <span className="hidden sm:inline">{t('documents.viewer.split', 'Split')}</span>
          </Button>

          {/* Markdown Only Button */}
          <Button
            variant={viewMode === 'markdown' ? 'secondary' : 'ghost'}
            size="sm"
            className="h-8"
            onClick={() => handleModeChange('markdown')}
            title={t('documents.viewer.markdownOnly', 'Markdown Only')}
          >
            <FileText className="h-4 w-4 mr-1.5" />
            <span className="hidden sm:inline">Markdown</span>
          </Button>
        </div>
      </div>

      {/* Content Area */}
      <div
        className={cn(
          'flex-1',
          // Responsive grid layout
          viewMode === 'split' ? 'flex flex-col lg:grid lg:grid-cols-2' : 'flex'
        )}
        style={{ height: `${height}px` }}
      >
        {/* PDF Panel */}
        {showPdf && (
          <div
            className={cn(
              'flex flex-col overflow-hidden',
              viewMode === 'split' 
                ? 'h-1/2 lg:h-full lg:border-r border-border' 
                : 'flex-1'
            )}
          >
            <PDFViewer
              file={pdfUrl}
              showToolbar={true}
              height={viewMode === 'split' ? height / 2 : height}
              className="flex-1"
            />
          </div>
        )}

        {/* Markdown Panel */}
        {showMarkdown && (
          <div
            className={cn(
              'flex flex-col overflow-hidden',
              viewMode === 'split' ? 'h-1/2 lg:h-full' : 'flex-1'
            )}
          >
            <MarkdownViewer
              content={markdown}
              showToolbar={true}
              height={viewMode === 'split' ? height / 2 : height}
              title={t('documents.viewer.extractedMarkdown', 'Extracted Markdown')}
              className="flex-1"
            />
          </div>
        )}
      </div>
    </div>
  );
}

export default PDFMarkdownSplitView;
