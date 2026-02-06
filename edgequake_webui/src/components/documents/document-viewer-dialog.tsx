/**
 * @module DocumentViewerDialog
 * @description Modal dialog for viewing PDF documents with side-by-side Markdown.
 * Full-featured document viewer with PDF rendering and extracted markdown display.
 *
 * @implements SPEC-002 - Document Viewer Dialog
 * @implements FEAT0741 - Full-screen document viewing dialog
 * @implements FEAT0742 - PDF and Markdown side-by-side display
 * @implements FEAT0743 - Download and share actions
 *
 * @enforces BR0741 - Accessible dialog with keyboard navigation
 * @enforces BR0742 - Responsive sizing for different screens
 *
 * @see {@link docs/features.md} FEAT0741-0743
 */
'use client';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { getPdfContent, getPdfDownloadUrl } from '@/lib/api/edgequake';
import { useQuery } from '@tanstack/react-query';
import {
    AlertCircle,
    Download,
    ExternalLink,
    FileText,
    Loader2,
} from 'lucide-react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { MarkdownViewer } from './markdown-viewer';
import { PDFViewer } from './pdf-viewer';
import { SideBySideViewer } from './side-by-side-viewer';

interface DocumentViewerDialogProps {
  /** PDF document ID */
  pdfId: string | null;
  /** Document title/filename */
  title?: string;
  /** Whether dialog is open */
  open: boolean;
  /** Called when dialog should close */
  onOpenChange: (open: boolean) => void;
}

/**
 * DocumentViewerDialog component for viewing PDF documents.
 *
 * Features:
 * - Full-screen modal for document viewing
 * - PDF viewer with pagination and zoom
 * - Side-by-side Markdown display
 * - Download and external link actions
 */
export function DocumentViewerDialog({
  pdfId,
  title,
  open,
  onOpenChange,
}: DocumentViewerDialogProps) {
  const { t } = useTranslation();

  // Fetch PDF content metadata (includes markdown)
  const { data: pdfContent, isLoading: isLoadingContent, error } = useQuery({
    queryKey: ['pdf-content', pdfId],
    queryFn: () => pdfId ? getPdfContent(pdfId) : Promise.resolve(null),
    enabled: !!pdfId && open,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });

  // Get PDF download URL
  const pdfUrl = useMemo(() => {
    if (!pdfId) return null;
    return getPdfDownloadUrl(pdfId);
  }, [pdfId]);

  // Handle download
  const handleDownload = () => {
    if (!pdfUrl || !pdfContent) return;
    
    // Open download in new tab (browser will handle the download)
    const link = document.createElement('a');
    link.href = pdfUrl;
    link.download = pdfContent.filename || 'document.pdf';
    link.target = '_blank';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    
    toast.success(t('documents.viewer.downloadStarted', 'Download started'));
  };

  // Handle open in new tab
  const handleOpenExternal = () => {
    if (!pdfUrl) return;
    window.open(pdfUrl, '_blank');
  };

  const displayTitle = title || pdfContent?.filename || t('documents.viewer.document', 'Document');
  const isPdf = pdfContent?.content_type === 'application/pdf';
  const hasMarkdown = !!pdfContent?.markdown_content;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[95vw] w-[95vw] max-h-[95vh] h-[95vh] p-0 gap-0">
        <DialogHeader className="px-4 py-3 border-b flex-shrink-0">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <FileText className="h-5 w-5 text-muted-foreground" />
              <div>
                <DialogTitle className="text-base">{displayTitle}</DialogTitle>
                {pdfContent && (
                  <DialogDescription className="text-xs">
                    {formatFileSize(pdfContent.file_size_bytes)}
                    {pdfContent.is_processed && ' • Processed'}
                  </DialogDescription>
                )}
              </div>
            </div>
            
            <div className="flex items-center gap-2">
              {pdfUrl && (
                <>
                  <Button
                    variant="outline"
                    size="sm"
                    className="h-8"
                    onClick={handleDownload}
                  >
                    <Download className="h-4 w-4 mr-1.5" />
                    {t('documents.viewer.download', 'Download')}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={handleOpenExternal}
                    title={t('documents.viewer.openInNewTab', 'Open in new tab')}
                  >
                    <ExternalLink className="h-4 w-4" />
                  </Button>
                </>
              )}
            </div>
          </div>
        </DialogHeader>

        <div className="flex-1 overflow-hidden">
          {/* Loading State */}
          {isLoadingContent && (
            <div className="flex items-center justify-center h-full">
              <div className="flex flex-col items-center gap-4">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                <p className="text-sm text-muted-foreground">
                  {t('documents.viewer.loading', 'Loading document...')}
                </p>
              </div>
            </div>
          )}

          {/* Error State */}
          {error && (
            <div className="flex items-center justify-center h-full">
              <div className="flex flex-col items-center gap-4 text-center p-8">
                <div className="rounded-full bg-destructive/10 p-4">
                  <FileText className="h-8 w-8 text-destructive" />
                </div>
                <p className="text-sm text-muted-foreground">
                  {t('documents.viewer.loadError', 'Failed to load document')}
                </p>
                <p className="text-xs text-destructive">
                  {error instanceof Error ? error.message : 'Unknown error'}
                </p>
              </div>
            </div>
          )}

          {/* Content */}
          {pdfContent && !isLoadingContent && (
            <>
              {/* If PDF with markdown, show side-by-side */}
              {isPdf && hasMarkdown && pdfUrl && (
                <SideBySideViewer
                  leftPanel={
                    <PDFViewer
                      file={pdfUrl}
                      showToolbar={true}
                      className="h-full"
                    />
                  }
                  rightPanel={
                    <MarkdownViewer
                      content={pdfContent.markdown_content}
                      showToolbar={false}
                      className="h-full"
                    />
                  }
                  height={window.innerHeight - 150}
                  leftTitle={t('documents.viewer.originalPdf', 'Original PDF')}
                  rightTitle={t('documents.viewer.extractedMarkdown', 'Extracted Markdown')}
                />
              )}

              {/* If PDF without markdown, show PDF with extraction status message */}
              {/* OODA-E2E-01: Show explicit message when markdown extraction failed/pending */}
              {isPdf && !hasMarkdown && pdfUrl && (
                <SideBySideViewer
                  leftPanel={
                    <PDFViewer
                      file={pdfUrl}
                      showToolbar={true}
                      className="h-full"
                    />
                  }
                  rightPanel={
                    <div className="flex items-center justify-center h-full bg-muted/30">
                      <div className="flex flex-col items-center gap-4 text-center p-8 max-w-md">
                        {pdfContent.is_processed ? (
                          <>
                            <div className="rounded-full bg-amber-500/10 p-4">
                              <AlertCircle className="h-8 w-8 text-amber-500" />
                            </div>
                            <div>
                              <p className="font-medium text-foreground mb-1">
                                {t('documents.viewer.extractionFailed', 'Markdown Extraction Failed')}
                              </p>
                              <p className="text-sm text-muted-foreground">
                                {t(
                                  'documents.viewer.extractionFailedDesc',
                                  'The PDF was processed but no markdown content was extracted. This usually means the PDF extraction library (libpdfium) is not available on the server. Please check the server logs for details.'
                                )}
                              </p>
                            </div>
                          </>
                        ) : (
                          <>
                            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                            <div>
                              <p className="font-medium text-foreground mb-1">
                                {t('documents.viewer.processing', 'Processing PDF...')}
                              </p>
                              <p className="text-sm text-muted-foreground">
                                {t(
                                  'documents.viewer.processingDesc',
                                  'The PDF is being processed. Markdown content will appear here once extraction is complete.'
                                )}
                              </p>
                            </div>
                          </>
                        )}
                      </div>
                    </div>
                  }
                  height={window.innerHeight - 150}
                  leftTitle={t('documents.viewer.originalPdf', 'Original PDF')}
                  rightTitle={t('documents.viewer.extractedMarkdown', 'Extracted Markdown')}
                />
              )}

              {/* If only markdown (processed text), show markdown */}
              {!isPdf && hasMarkdown && (
                <MarkdownViewer
                  content={pdfContent.markdown_content}
                  showToolbar={true}
                  height={window.innerHeight - 150}
                  className="w-full"
                />
              )}

              {/* No content available */}
              {!isPdf && !hasMarkdown && (
                <div className="flex items-center justify-center h-full">
                  <div className="flex flex-col items-center gap-4 text-center p-8">
                    <FileText className="h-12 w-12 text-muted-foreground opacity-50" />
                    <p className="text-sm text-muted-foreground">
                      {t('documents.viewer.noContent', 'No viewable content available')}
                    </p>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

/**
 * Format file size to human-readable string.
 */
function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

export default DocumentViewerDialog;
