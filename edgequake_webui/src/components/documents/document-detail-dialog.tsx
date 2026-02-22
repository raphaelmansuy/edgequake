/**
 * @module DocumentDetailDialog
 * @description Modal dialog showing full document metadata and actions.
 * Displays status, statistics, chunks, and entities with reprocess option.
 * 
 * @implements UC0010 - User views document details
 * @implements FEAT0631 - Document metadata display
 * @implements FEAT0632 - Chunk/entity statistics
 * @implements SPEC-002 - Unified Ingestion Pipeline (uses current_stage)
 * @implements FEAT0731 - PDF+Markdown split view for PDF documents (OODA-82)
 * 
 * @enforces BR0621 - All document fields visible
 * @enforces BR0302 - Reprocess action available for failed docs
 * 
 * @see {@link docs/use_cases.md} UC0010
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { Document } from '@/types';
import { formatDistanceToNow } from 'date-fns';
import {
    Calendar,
    Clock,
    Copy,
    FileText,
    Hash,
    Link2,
    Network,
    RotateCcw,
    Tag
} from 'lucide-react';
import Link from 'next/link';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { PDFMarkdownSplitView } from './pdf-markdown-split-view';
import { StatusBadge as UnifiedStatusBadge, getDocumentDisplayStatus } from './status-badge';

interface DocumentDetailDialogProps {
  document: Document | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onReprocess?: (id: string) => void;
}

/**
 * SPEC-002: Use unified StatusBadge component that prefers current_stage.
 * This inline wrapper exists for backward compatibility with existing usages.
 */
function StatusBadge({ status, document }: { status: Document['status']; document?: Document }) {
  // SPEC-002: If document is available, use unified current_stage
  if (document) {
    const displayStatus = getDocumentDisplayStatus(document);
    return <UnifiedStatusBadge status={displayStatus} />;
  }
  
  // Legacy fallback for simple status strings
  const statusConfig = {
    pending: { label: 'Pending', variant: 'secondary' as const },
    processing: { label: 'Processing', variant: 'default' as const },
    completed: { label: 'Completed', variant: 'default' as const },
    indexed: { label: 'Indexed', variant: 'default' as const },
    failed: { label: 'Failed', variant: 'destructive' as const },
    partial_failure: { label: 'Partial Failure', variant: 'destructive' as const },
    cancelled: { label: 'Cancelled', variant: 'outline' as const },
  };

  // Handle 'indexed' as 'completed' for display purposes
  const key = status || 'pending';
  const config = statusConfig[key] || statusConfig.pending;
  return <Badge variant={config.variant}>{config.label}</Badge>;
}

function MetadataItem({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof FileText;
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-3 py-2">
      <Icon className="h-4 w-4 text-muted-foreground mt-0.5" />
      <div className="flex-1 min-w-0">
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="text-sm font-medium truncate">{value}</p>
      </div>
    </div>
  );
}

/**
 * Dialog showing detailed information about a document
 */
export function DocumentDetailDialog({
  document,
  open,
  onOpenChange,
  onReprocess,
}: DocumentDetailDialogProps) {
  const { t } = useTranslation();

  if (!document) return null;

  const handleCopyId = () => {
    navigator.clipboard.writeText(document.id);
    toast.success(t('common.copied', 'Copied!'));
  };

  const formatDate = (date: string | Date | undefined) => {
    if (!date) return 'N/A';
    const d = typeof date === 'string' ? new Date(date) : date;
    return formatDistanceToNow(d, { addSuffix: true });
  };

  // OODA-82: Check if this is a PDF-origin document (has pdf_id)
  const hasPdfSource = Boolean(document.pdf_id);
  // Construct PDF URL for viewing
  const pdfUrl = hasPdfSource ? `/api/v1/documents/pdf/${document.pdf_id}/download` : '';

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={hasPdfSource ? "sm:max-w-4xl max-h-[90vh]" : "sm:max-w-2xl max-h-[85vh]"}>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            {document.title || 'Untitled Document'}
          </DialogTitle>
          <DialogDescription className="flex items-center gap-2">
            <StatusBadge status={document.status} document={document} />
            <span className="text-muted-foreground">·</span>
            <span>{formatDate(document.created_at)}</span>
          </DialogDescription>
        </DialogHeader>

        <Tabs defaultValue="overview" className="mt-4">
          {/* OODA-82: Dynamic grid columns based on whether PDF source exists */}
          <TabsList className={`grid w-full ${hasPdfSource ? 'grid-cols-4' : 'grid-cols-3'}`}>
            <TabsTrigger value="overview">
              {t('documents.details.overview', 'Overview')}
            </TabsTrigger>
            {/* OODA-82: Source tab for PDF documents showing PDF+Markdown side-by-side */}
            {hasPdfSource && (
              <TabsTrigger value="source">
                {t('documents.details.source', 'Source')}
              </TabsTrigger>
            )}
            <TabsTrigger value="content">
              {t('documents.details.content', 'Content')}
            </TabsTrigger>
            <TabsTrigger value="entities">
              {t('documents.details.entities', 'Entities')}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="overview" className="mt-4 space-y-4">
            {/* Metadata Grid */}
            <div className="grid grid-cols-2 gap-4 p-4 bg-muted/50 rounded-lg">
              <MetadataItem
                icon={Hash}
                label={t('documents.details.id', 'Document ID')}
                value={
                  <span className="flex items-center gap-1">
                    <code className="text-xs">{document.id.slice(0, 12)}...</code>
                    <Button variant="ghost" size="icon" className="h-6 w-6" onClick={handleCopyId}>
                      <Copy className="h-3 w-3" />
                    </Button>
                  </span>
                }
              />
              <MetadataItem
                icon={Tag}
                label={t('documents.details.status', 'Status')}
                value={<StatusBadge status={document.status} document={document} />}
              />
              <MetadataItem
                icon={Calendar}
                label={t('documents.details.created', 'Created')}
                value={formatDate(document.created_at)}
              />
              <MetadataItem
                icon={Clock}
                label={t('documents.details.updated', 'Last Updated')}
                value={formatDate(document.updated_at)}
              />
              <MetadataItem
                icon={Link2}
                label={t('documents.details.entities', 'Entities Extracted')}
                value={document.entity_count ?? 0}
              />
              <MetadataItem
                icon={FileText}
                label={t('documents.details.chunks', 'Chunks')}
                value={document.chunk_count ?? 0}
              />
            </div>

            {/* Error message if failed or cancelled */}
            {(document.status === 'failed' || document.status === 'partial_failure' || document.status === 'cancelled') && document.error_message && (
              <div className={`p-4 ${document.status === 'cancelled' ? 'bg-gray-500/10 border-gray-200' : 'bg-destructive/10 border-destructive/20'} border rounded-lg`}>
                <h4 className={`text-sm font-medium ${document.status === 'cancelled' ? 'text-gray-600 dark:text-gray-400' : 'text-destructive'} mb-1`}>
                  {document.status === 'cancelled' 
                    ? t('documents.details.cancelled', 'Cancelled')
                    : t('documents.details.error', 'Error')}
                </h4>
                <p className={`text-xs ${document.status === 'cancelled' ? 'text-gray-500' : 'text-destructive/80'}`}>{document.error_message}</p>
              </div>
            )}

            {/* Cancelled banner when no error message */}
            {document.status === 'cancelled' && !document.error_message && (
              <div className="p-4 bg-gray-500/10 border border-gray-200 rounded-lg">
                <h4 className="text-sm font-medium text-gray-600 dark:text-gray-400 mb-1">
                  {t('documents.details.cancelled', 'Cancelled')}
                </h4>
                <p className="text-xs text-gray-500">
                  {t('documents.cancelled.hint', 'You can reprocess this document to resume extraction.')}
                </p>
              </div>
            )}

            {/* Actions */}
            <div className="flex gap-2 pt-2">
              {(document.status === 'failed' || document.status === 'partial_failure' || document.status === 'cancelled') && onReprocess && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onReprocess(document.id)}
                >
                  <RotateCcw className="h-4 w-4 mr-2" />
                  {t('documents.actions.reprocess', 'Reprocess')}
                </Button>
              )}
            </div>
          </TabsContent>

          {/* OODA-82: Source tab for PDF documents - shows PDF+Markdown side by side */}
          {hasPdfSource && (
            <TabsContent value="source" className="mt-4">
              <PDFMarkdownSplitView
                pdfUrl={pdfUrl}
                markdown={document.content ?? null}
                height={450}
              />
            </TabsContent>
          )}

          <TabsContent value="content" className="mt-4">
            <ScrollArea className="h-[300px] rounded-lg border p-4">
              {document.content ? (
                <pre className="text-sm whitespace-pre-wrap font-mono">
                  {document.content}
                </pre>
              ) : (
                <p className="text-muted-foreground text-sm">
                  {t('documents.details.noContent', 'No content available')}
                </p>
              )}
            </ScrollArea>
          </TabsContent>

          <TabsContent value="entities" className="mt-4">
            <ScrollArea className="h-[300px] rounded-lg border p-4">
              {document.entity_count && document.entity_count > 0 ? (
                <div className="text-center py-8">
                  <p className="text-sm text-muted-foreground mb-2">
                    {t('documents.details.entitiesExtracted', '{{count}} entities extracted', { count: document.entity_count })}
                  </p>
                  <p className="text-xs text-muted-foreground mb-4">
                    {t('documents.details.viewInGraphHint', 'View in Knowledge Graph for detailed entity information')}
                  </p>
                  <Button asChild variant="outline" size="sm">
                    <Link href={`/graph?document=${encodeURIComponent(document.id)}`}>
                      <Network className="h-4 w-4 mr-2" />
                      {t('documents.actions.viewInGraph', 'View in Graph')}
                    </Link>
                  </Button>
                </div>
              ) : (
                <p className="text-muted-foreground text-sm text-center py-8">
                  {t('documents.details.noEntities', 'No entities extracted yet')}
                </p>
              )}
            </ScrollArea>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}

export default DocumentDetailDialog;
