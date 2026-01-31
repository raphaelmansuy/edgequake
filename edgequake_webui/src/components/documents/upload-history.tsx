/**
 * @module UploadHistory
 * @description Component for displaying upload history with filtering and search.
 * Shows past completed and failed uploads with timing and stats.
 *
 * @implements OODA-24: Upload history table
 * @implements UC0711: User views upload history
 * @implements FEAT0608: Historical tracking persists across sessions
 *
 * @enforces BR0708: Maintain last 100 uploads in history
 * @enforces BR0709: Allow filter by status (success/failed)
 *
 * @see {@link specs/001-upload-pdf.md} Mission specification
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useIngestionStore } from '@/stores/use-ingestion-store';
import {
    AlertCircle,
    CheckCircle,
    Clock,
    ExternalLink,
    FileText,
    RefreshCw,
    Search,
    Trash2,
    XCircle,
} from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

// ============================================================================
// Types
// ============================================================================

type HistoryFilter = 'all' | 'success' | 'failed';

interface HistoryItem {
  type: 'success' | 'failed';
  trackId: string;
  documentId?: string;
  documentName?: string;
  timestamp: Date;
  durationMs?: number;
  chunks?: number;
  entities?: number;
  relationships?: number;
  error?: string;
}

// ============================================================================
// Component
// ============================================================================

export interface UploadHistoryProps {
  /** Max number of items to display */
  maxItems?: number;
  /** Whether to show in compact mode */
  compact?: boolean;
  /** 
   * Callback when user clicks retry on a failed item.
   * OODA-16: Enhanced to pass documentId for reprocessing.
   */
  onRetry?: (trackId: string, documentId?: string) => void;
}

export function UploadHistory({
  maxItems = 20,
  compact = false,
  onRetry,
}: UploadHistoryProps) {
  const { t } = useTranslation();
  const router = useRouter();
  
  // Get data from store
  const { completedJobs, failedJobs, clearCompletedJobs, clearFailedJob, clearAllFailedJobs } = useIngestionStore();

  // Local state
  const [searchQuery, setSearchQuery] = useState('');
  const [filter, setFilter] = useState<HistoryFilter>('all');

  // Combine and transform history items
  const historyItems = useMemo((): HistoryItem[] => {
    const items: HistoryItem[] = [];

    // Add completed jobs
    for (const job of completedJobs) {
      items.push({
        type: 'success',
        trackId: job.track_id,
        documentId: job.document_id,
        durationMs: job.duration_ms,
        chunks: job.chunks,
        entities: job.entities,
        relationships: job.relationships,
        timestamp: new Date(), // Note: would need completed_at from store
      });
    }

    // Add failed jobs
    for (const [trackId, error] of failedJobs) {
      items.push({
        type: 'failed',
        trackId,
        error: error.message,
        timestamp: new Date(), // Note: would need failed_at from store
      });
    }

    return items;
  }, [completedJobs, failedJobs]);

  // Filter and search
  const filteredItems = useMemo(() => {
    let items = historyItems;

    // Apply filter
    if (filter !== 'all') {
      items = items.filter((item) => item.type === filter);
    }

    // Apply search
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      items = items.filter(
        (item) =>
          item.trackId.toLowerCase().includes(query) ||
          item.documentId?.toLowerCase().includes(query) ||
          item.documentName?.toLowerCase().includes(query)
      );
    }

    // Limit and sort by timestamp (newest first)
    return items
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
      .slice(0, maxItems);
  }, [historyItems, filter, searchQuery, maxItems]);

  // Stats
  const successCount = historyItems.filter((i) => i.type === 'success').length;
  const failedCount = historyItems.filter((i) => i.type === 'failed').length;
  const successRate = historyItems.length > 0
    ? Math.round((successCount / historyItems.length) * 100)
    : 0;

  // Format duration
  const formatDuration = (ms?: number) => {
    if (!ms) return '-';
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  };

  // Handle view document
  const handleViewDocument = (documentId?: string) => {
    if (documentId) {
      router.push(`/documents/${documentId}`);
    }
  };

  // Handle clear all
  const handleClearAll = () => {
    clearCompletedJobs();
    clearAllFailedJobs();
  };

  if (compact) {
    return (
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">
            {t('documents.history.title', 'Upload History')}
          </span>
          <Badge 
            variant={successRate >= 90 ? 'secondary' : 'destructive'}
            className={successRate >= 90 ? 'bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-300' : ''}
          >
            {successRate}% success
          </Badge>
        </div>
        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          <span className="flex items-center gap-1">
            <CheckCircle className="h-3 w-3 text-green-500" />
            {successCount} {t('documents.history.succeeded', 'succeeded')}
          </span>
          <span className="flex items-center gap-1">
            <XCircle className="h-3 w-3 text-red-500" />
            {failedCount} {t('documents.history.failed', 'failed')}
          </span>
        </div>
      </div>
    );
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <Clock className="h-5 w-5" />
            {t('documents.history.title', 'Upload History')}
          </CardTitle>
          <div className="flex items-center gap-2">
            <Badge 
              variant={successRate >= 90 ? 'secondary' : 'destructive'}
              className={successRate >= 90 ? 'bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-300' : ''}
            >
              {successRate}% success rate
            </Badge>
            {historyItems.length > 0 && (
              <Button variant="ghost" size="sm" onClick={handleClearAll}>
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Search and Filter */}
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-2 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder={t('documents.history.search', 'Search by ID or name...')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 h-8"
            />
          </div>
          <div className="flex items-center gap-1">
            <Button
              variant={filter === 'all' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setFilter('all')}
            >
              {t('documents.history.all', 'All')} ({historyItems.length})
            </Button>
            <Button
              variant={filter === 'success' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setFilter('success')}
              className="text-green-600"
            >
              <CheckCircle className="h-3 w-3 mr-1" />
              {successCount}
            </Button>
            <Button
              variant={filter === 'failed' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setFilter('failed')}
              className="text-red-600"
            >
              <XCircle className="h-3 w-3 mr-1" />
              {failedCount}
            </Button>
          </div>
        </div>

        {/* History Table */}
        {filteredItems.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
            <FileText className="h-8 w-8 mb-2" />
            <p className="text-sm">
              {historyItems.length === 0
                ? t('documents.history.empty', 'No upload history yet')
                : t('documents.history.noResults', 'No results match your search')}
            </p>
          </div>
        ) : (
          <ScrollArea className="max-h-64">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[40px]"></TableHead>
                  <TableHead>{t('documents.history.document', 'Document')}</TableHead>
                  <TableHead className="text-right">{t('documents.history.duration', 'Duration')}</TableHead>
                  <TableHead className="text-right">{t('documents.history.entities', 'Entities')}</TableHead>
                  <TableHead className="w-[80px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredItems.map((item) => (
                  <TableRow key={item.trackId}>
                    <TableCell>
                      {item.type === 'success' ? (
                        <CheckCircle className="h-4 w-4 text-green-500" />
                      ) : (
                        <TooltipProvider>
                          <Tooltip>
                            <TooltipTrigger>
                              <AlertCircle className="h-4 w-4 text-red-500" />
                            </TooltipTrigger>
                            <TooltipContent>
                              <p className="max-w-xs">{item.error}</p>
                            </TooltipContent>
                          </Tooltip>
                        </TooltipProvider>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-col">
                        <span className="font-mono text-xs truncate max-w-[150px]">
                          {item.documentId?.slice(0, 8) || item.trackId.slice(0, 8)}...
                        </span>
                        {item.documentName && (
                          <span className="text-xs text-muted-foreground truncate max-w-[150px]">
                            {item.documentName}
                          </span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-right text-xs font-mono">
                      {formatDuration(item.durationMs)}
                    </TableCell>
                    <TableCell className="text-right text-xs">
                      {item.entities ?? '-'}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1 justify-end">
                        {item.type === 'success' && item.documentId && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6"
                            onClick={() => handleViewDocument(item.documentId)}
                          >
                            <ExternalLink className="h-3 w-3" />
                          </Button>
                        )}
                        {item.type === 'failed' && onRetry && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6"
                            onClick={() => onRetry(item.trackId, item.documentId)}
                            title="Retry processing"
                          >
                            <RefreshCw className="h-3 w-3" />
                          </Button>
                        )}
                        {item.type === 'failed' && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6 text-red-500"
                            onClick={() => clearFailedJob(item.trackId)}
                          >
                            <Trash2 className="h-3 w-3" />
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}

export default UploadHistory;
