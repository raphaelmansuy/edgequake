'use client';

import { RightPanel } from '@/components/layout/right-panel';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import {
    deleteAllDocuments,
    deleteDocument,
    getDocuments,
    getPipelineStatus,
    reprocessDocument,
    uploadDocument,
} from '@/lib/api/edgequake';
import { cn } from '@/lib/utils';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { Document } from '@/types';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
    AlertCircle,
    CheckCircle,
    Clock,
    Eye,
    FileSearch,
    FileText,
    Loader2,
    MoreVertical,
    RefreshCw,
    Search,
    Sparkles,
    Trash2,
    Upload,
    X,
    XCircle,
} from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useCallback, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { BatchProgressCard } from './batch-progress-card';
import { ClearDocumentsDialog } from './clear-documents-dialog';
import { DocumentFilters, type DocStatus, type SortDirection, type SortField } from './document-filters';
import { DocumentPreviewPanel } from './document-preview-panel';
import { PaginationControls } from './pagination-controls';
import { PipelineStatusDialog } from './pipeline-status-dialog';
import { ReprocessFailedButton } from './reprocess-failed-button';
import { ResetDocumentStatusButton } from './reset-document-status-button';

// Track upload progress and errors for files
interface UploadingFile {
  file: File;
  progress: number;
  status: 'pending' | 'reading' | 'uploading' | 'extracting' | 'success' | 'error';
  error?: string;
  phase?: string; // Human-readable phase description
}

const statusConfig = {
  pending: { icon: Clock, color: 'bg-yellow-500', label: 'Pending', animate: false },
  processing: { icon: Loader2, color: 'bg-blue-500', label: 'Processing', animate: true },
  completed: { icon: CheckCircle, color: 'bg-green-500', label: 'Completed', animate: false },
  indexed: { icon: CheckCircle, color: 'bg-green-500', label: 'Indexed', animate: false },
  failed: { icon: XCircle, color: 'bg-red-500', label: 'Failed', animate: false },
} as const;

type DocumentStatus = keyof typeof statusConfig;

function StatusBadge({ status }: { status: DocumentStatus }) {
  const config = statusConfig[status] || statusConfig.completed;
  const Icon = config.icon;

  return (
    <Badge variant="outline" className="gap-1">
      <Icon className={`h-3 w-3 ${config.animate ? 'animate-spin' : ''}`} />
      {config.label}
    </Badge>
  );
}

export function DocumentManager() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const router = useRouter();
  
  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  
  // Bulk selection state
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  
  // Selected document for preview panel
  const [selectedDocument, setSelectedDocument] = useState<Document | null>(null);
  const [previewPanelOpen, setPreviewPanelOpen] = useState(false);
  
  // Search state
  const [searchQuery, setSearchQuery] = useState('');
  
  // Pagination state
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  
  // Filter and sort state
  const [statusFilter, setStatusFilter] = useState<DocStatus>('all');
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  
  // Pipeline status dialog state
  const [pipelineDialogOpen, setPipelineDialogOpen] = useState(false);

  // Upload progress tracking state
  const [uploadingFiles, setUploadingFiles] = useState<UploadingFile[]>([]);
  const [isUploading, setIsUploading] = useState(false);
  
  // Track ID for batch progress (Phase 2) - async processing enabled
  const [activeTrackId, setActiveTrackId] = useState<string | null>(null);

  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, currentPage, pageSize, statusFilter],
    queryFn: () => getDocuments({ 
      page: currentPage, 
      page_size: pageSize,
      status: statusFilter === 'all' ? undefined : statusFilter,
    }),
    refetchInterval: 5000, // Poll for status updates
  });
  
  // Pipeline status query
  const { data: pipelineStatus } = useQuery({
    queryKey: ['pipeline-status'],
    queryFn: getPipelineStatus,
    refetchInterval: 2000,
  });

  // Enhanced upload handler with progress tracking
  const handleFilesUpload = useCallback(
    async (files: File[]) => {
      if (files.length === 0) return;
      
      setIsUploading(true);
      
      // Generate a shared track_id for this batch
      const trackId = `upload_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
      
      // Initialize upload state for all files
      const initialFiles: UploadingFile[] = files.map((file) => ({
        file,
        progress: 0,
        status: 'pending' as const,
        phase: 'Waiting...',
      }));
      setUploadingFiles(initialFiles);

      // Show loading toast
      const toastId = toast.loading(
        t('documents.upload.inProgress', { count: files.length }) || `Uploading ${files.length} file(s)...`,
        { duration: Infinity }
      );

      let successCount = 0;
      let errorCount = 0;

      // Process files sequentially for better feedback
      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        
        // Phase 1: Reading file
        setUploadingFiles((prev) =>
          prev.map((f, idx) =>
            idx === i ? { ...f, status: 'reading' as const, progress: 10, phase: t('documents.upload.reading', 'Reading file...') } : f
          )
        );

        try {
          // Read file content
          const text = await file.text();
          
          // Phase 2: Uploading to server
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { ...f, status: 'uploading' as const, progress: 40, phase: t('documents.upload.uploading', 'Uploading to server...') } : f
            )
          );

          // Upload to server with filename as title
          // Using async processing - documents are queued and processed in background
          // by the WorkerPool. Status updates via track_id polling.
          const response = await uploadDocument({ 
            content: text, 
            source_type: 'text',
            title: file.name, // Use filename as title
            async_processing: true, // Use async processing (worker pool enabled)
            track_id: trackId, // Track ID for grouping
          });
          
          // Check for duplicate (Phase 4)
          if (response.duplicate_of) {
            // Show duplicate warning
            toast.warning(
              t('documents.upload.duplicate', '{{name}} is a duplicate (existing: {{id}})', {
                name: file.name,
                id: response.duplicate_of.slice(0, 8),
              }),
              { duration: 4000 }
            );
            
            // Mark as duplicate (treat as success but with warning)
            setUploadingFiles((prev) =>
              prev.map((f, idx) =>
                idx === i ? { 
                  ...f, 
                  status: 'success' as const, 
                  progress: 100, 
                  phase: t('documents.upload.duplicateSkipped', 'Duplicate (skipped)'),
                } : f
              )
            );
            successCount++;
            continue; // Skip to next file
          }
          
          // Phase 3: Extraction queued
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { 
                ...f, 
                status: 'extracting' as const, 
                progress: 80, 
                phase: response.task_id 
                  ? t('documents.upload.queued', 'Queued for extraction (Task: {{taskId}})', { taskId: response.task_id.slice(0, 8) })
                  : t('documents.upload.extracting', 'Processing...'),
              } : f
            )
          );
          
          // Brief delay to show extraction phase
          await new Promise(resolve => setTimeout(resolve, 300));
          
          // Mark as complete
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { ...f, status: 'success' as const, progress: 100, phase: t('documents.upload.complete', 'Complete!') } : f
            )
          );
          
          successCount++;
        } catch (error) {
          // Mark as error
          const errorMessage = error instanceof Error ? error.message : 'Upload failed';
          setUploadingFiles((prev) =>
            prev.map((f, idx) =>
              idx === i ? { ...f, status: 'error' as const, progress: 100, error: errorMessage, phase: t('common.failed', 'Failed') } : f
            )
          );
          
          errorCount++;
        }
      }

      // Update toast with final result
      if (errorCount === 0) {
        toast.success(
          t('documents.upload.success', { count: successCount }) || `Successfully uploaded ${successCount} file(s)`,
          { 
            id: toastId, 
            duration: 5000,
            action: {
              label: t('documents.upload.viewInGraph', 'View in Graph'),
              onClick: () => router.push('/graph'),
            },
          }
        );
      } else if (successCount === 0) {
        toast.error(
          t('documents.upload.allFailed', { count: errorCount }) || `All ${errorCount} file(s) failed to upload`,
          { 
            id: toastId, 
            duration: 5000,
            action: {
              label: t('common.retry', 'Retry'),
              onClick: () => {
                // Reset and allow user to try again
                setUploadingFiles([]);
              },
            },
          }
        );
      } else {
        toast.warning(
          t('documents.upload.partial', { success: successCount, failed: errorCount }) || 
            `Uploaded ${successCount} file(s), ${errorCount} failed`,
          { 
            id: toastId, 
            duration: 5000,
            action: {
              label: t('documents.upload.viewInGraph', 'View in Graph'),
              onClick: () => router.push('/graph'),
            },
          }
        );
      }

      // Refresh documents list
      queryClient.invalidateQueries({ queryKey: ['documents'] });
      setIsUploading(false);

      // Set the active track ID to show batch progress card (for async processing)
      if (successCount > 0) {
        setActiveTrackId(trackId);
      }

      // Clear upload list after a delay
      setTimeout(() => {
        setUploadingFiles([]);
      }, 3000);
    },
    [queryClient, t, router]
  );

  // Remove a file from the upload list
  const removeUploadingFile = useCallback((index: number) => {
    setUploadingFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  // Maximum file size: 10MB
  const MAX_FILE_SIZE = 10 * 1024 * 1024;

  const onDrop = useCallback(
    async (acceptedFiles: File[], fileRejections: readonly { file: File; errors: readonly { code: string; message: string }[] }[]) => {
      // Handle rejected files (too large or wrong type)
      for (const rejection of fileRejections) {
        const errorMessages = rejection.errors.map(e => {
          if (e.code === 'file-too-large') {
            const sizeMB = (rejection.file.size / (1024 * 1024)).toFixed(2);
            return t('documents.upload.fileTooLarge', 'File "{{name}}" is too large ({{size}}MB). Maximum size is 10MB.', {
              name: rejection.file.name,
              size: sizeMB,
            });
          }
          if (e.code === 'file-invalid-type') {
            return t('documents.upload.invalidType', 'File "{{name}}" has an unsupported format. Supported: TXT, MD, JSON.', {
              name: rejection.file.name,
            });
          }
          return e.message;
        }).join(', ');
        
        toast.error(errorMessages);
      }
      
      // Process accepted files
      if (acceptedFiles.length > 0) {
        await handleFilesUpload(acceptedFiles);
      }
    },
    [handleFilesUpload, t]
  );

  const deleteMutation = useMutation({
    mutationFn: deleteDocument,
    onSuccess: () => {
      toast.success(t('documents.delete.success', 'Document deleted'), {
        duration: 4000,
        description: t('documents.delete.successDesc', 'The document has been permanently removed.'),
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.delete.failed', 'Delete failed'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => {
            // User can retry from the UI
          },
        },
      });
    },
  });

  const deleteAllMutation = useMutation({
    mutationFn: deleteAllDocuments,
    onSuccess: (data) => {
      toast.success(t('documents.deleteAll.success', { count: data.deleted_count }) || `Deleted ${data.deleted_count} documents`, {
        duration: 4000,
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.deleteAll.failed', 'Delete all failed'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => deleteAllMutation.mutate(),
        },
      });
    },
  });

  const reprocessMutation = useMutation({
    mutationFn: reprocessDocument,
    onSuccess: () => {
      toast.success(t('documents.reprocess.success', 'Document queued for reprocessing'), {
        duration: 4000,
        action: {
          label: t('documents.viewStatus', 'View Status'),
          onClick: () => setPipelineDialogOpen(true),
        },
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.reprocess.failed', 'Reprocess failed'), {
        description: error instanceof Error ? error.message : t('common.unknownError', 'Unknown error'),
        action: {
          label: t('common.retry', 'Retry'),
          onClick: () => {
            // User can retry from the UI
          },
        },
      });
    },
  });

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: {
      'text/plain': ['.txt'],
      'text/markdown': ['.md'],
      'application/json': ['.json'],
    },
    maxSize: MAX_FILE_SIZE, // 10MB limit
  });

  // Filter documents client-side (fallback when backend doesn't support filtering)
  const filterDocuments = (docs: Document[]): Document[] => {
    let filtered = docs;
    
    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase().trim();
      filtered = filtered.filter((doc) => {
        const title = doc.title?.toLowerCase() || '';
        const fileName = doc.file_name?.toLowerCase() || '';
        return title.includes(query) || fileName.includes(query) || doc.id.includes(query);
      });
    }
    
    // Apply status filter
    if (statusFilter !== 'all') {
      filtered = filtered.filter((doc) => {
        const docStatus = doc.status || 'completed';
        return docStatus === statusFilter;
      });
    }
    
    return filtered;
  };

  // Sort documents client-side for now
  const sortDocuments = (docs: Document[]): Document[] => {
    return [...docs].sort((a, b) => {
      let aVal: string | number | Date = '';
      let bVal: string | number | Date = '';
      
      switch (sortField) {
        case 'title':
          aVal = a.title || a.file_name || a.id;
          bVal = b.title || b.file_name || b.id;
          break;
        case 'created_at':
          aVal = new Date(a.created_at || 0);
          bVal = new Date(b.created_at || 0);
          break;
        case 'status':
          aVal = a.status || '';
          bVal = b.status || '';
          break;
        case 'entity_count':
          aVal = a.entity_count ?? a.chunk_count ?? 0;
          bVal = b.entity_count ?? b.chunk_count ?? 0;
          break;
      }
      
      if (aVal < bVal) return sortDirection === 'asc' ? -1 : 1;
      if (aVal > bVal) return sortDirection === 'asc' ? 1 : -1;
      return 0;
    });
  };

  const documents = sortDocuments(filterDocuments(data?.items || []));
  const allDocuments = data?.items || [];
  const totalPages = Math.ceil(documents.length / pageSize);
  const totalCount = documents.length;
  
  // Use server-side status counts from API response (more efficient)
  // Fall back to client-side calculation if not available
  const serverStatusCounts = data?.status_counts;
  const statusCounts: Record<DocStatus, number> = serverStatusCounts ? {
    all: allDocuments.length,
    pending: serverStatusCounts.pending,
    processing: serverStatusCounts.processing,
    completed: serverStatusCounts.completed,
    failed: serverStatusCounts.failed,
  } : {
    all: allDocuments.length,
    pending: allDocuments.filter((d) => d.status === 'pending').length,
    processing: allDocuments.filter((d) => d.status === 'processing').length,
    completed: allDocuments.filter((d) => !d.status || d.status === 'completed' || d.status === 'indexed').length,
    failed: allDocuments.filter((d) => d.status === 'failed').length,
  };

  // Bulk selection handlers
  const handleSelectAll = useCallback((checked: boolean) => {
    if (checked) {
      setSelectedIds(new Set(documents.map(d => d.id)));
    } else {
      setSelectedIds(new Set());
    }
  }, [documents]);

  const handleSelectOne = useCallback((docId: string, checked: boolean) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (checked) {
        next.add(docId);
      } else {
        next.delete(docId);
      }
      return next;
    });
  }, []);

  const handleBulkDelete = useCallback(async () => {
    const idsToDelete = Array.from(selectedIds);
    let successCount = 0;
    let errorCount = 0;

    for (const id of idsToDelete) {
      try {
        await deleteDocument(id);
        successCount++;
      } catch {
        errorCount++;
      }
    }

    if (successCount > 0) {
      toast.success(t('documents.bulk.deleteSuccess', { count: successCount }) || `Deleted ${successCount} document(s)`);
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    }
    if (errorCount > 0) {
      toast.error(t('documents.bulk.deleteFailed', { count: errorCount }) || `Failed to delete ${errorCount} document(s)`);
    }
    setSelectedIds(new Set());
  }, [selectedIds, queryClient, t]);

  const handleBulkReprocess = useCallback(async () => {
    const idsToReprocess = Array.from(selectedIds);
    let successCount = 0;
    let errorCount = 0;

    for (const id of idsToReprocess) {
      try {
        await reprocessDocument(id);
        successCount++;
      } catch {
        errorCount++;
      }
    }

    if (successCount > 0) {
      toast.success(t('documents.bulk.reprocessSuccess', { count: successCount }) || `Queued ${successCount} document(s) for reprocessing`);
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    }
    if (errorCount > 0) {
      toast.error(t('documents.bulk.reprocessFailed', { count: errorCount }) || `Failed to queue ${errorCount} document(s)`);
    }
    setSelectedIds(new Set());
  }, [selectedIds, queryClient, t]);

  // Document selection for preview panel
  const handleDocumentClick = useCallback((doc: Document) => {
    setSelectedDocument(doc);
    setPreviewPanelOpen(true);
  }, []);

  const handlePreviewClose = useCallback(() => {
    setSelectedDocument(null);
    setPreviewPanelOpen(false);
  }, []);

  const handleViewInGraph = useCallback((doc: Document) => {
    router.push(`/graph?entity=${encodeURIComponent(doc.id)}`);
  }, [router]);

  if (isError) {
    return (
      <div className="p-6">
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Error loading documents</AlertTitle>
          <AlertDescription>
            {error instanceof Error ? error.message : 'Failed to load documents'}
            <Button variant="link" className="ml-2 p-0" onClick={() => refetch()}>
              Try again
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Main Content - Flex column for proper scroll zones */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* Fixed Header Zone */}
        <div className="shrink-0 px-4 pt-4 space-y-3 bg-background">
          {/* Header - Compact */}
          <header className="flex items-center justify-between gap-3 flex-wrap">
            <div className="space-y-0.5">
              <h1 className="text-xl font-semibold tracking-tight">{t('documents.title')}</h1>
              <p className="text-sm text-muted-foreground">
                {t('documents.subtitle')}
              </p>
            </div>
          <div className="flex items-center gap-2 flex-wrap">
            {/* Pipeline Status */}
            {pipelineStatus?.is_busy && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPipelineDialogOpen(true)}
                className="gap-1 text-orange-500"
              >
                <Loader2 className="h-4 w-4 animate-spin" />
                {t('pipeline.busy')}
              </Button>
            )}
            <PipelineStatusDialog
              open={pipelineDialogOpen}
              onOpenChange={setPipelineDialogOpen}
            />
            
            {/* Reprocess Failed Button (GAP-UI-002) */}
            <ReprocessFailedButton
              failedCount={statusCounts.failed}
              onReprocessStarted={(trackId) => {
                setActiveTrackId(trackId);
                setPipelineDialogOpen(true);
              }}
            />
          
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4 mr-1" />
            {t('documents.refresh')}
          </Button>
          
          {/* Clear Documents Dialog (GAP-UI-009) */}
          <ClearDocumentsDialog
            documentCount={totalCount}
            onCleared={() => refetch()}
          />
        </div>
      </header>
      
      {/* Search and Filters */}
      <div className="flex flex-col sm:flex-row sm:items-center gap-3 pb-3 border-b">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={t('documents.search.placeholder', 'Search documents...')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 h-9"
          />
        </div>
        <DocumentFilters
          status={statusFilter}
          onStatusChange={setStatusFilter}
          sortField={sortField}
          onSortFieldChange={setSortField}
          sortDirection={sortDirection}
          onSortDirectionChange={setSortDirection}
          statusCounts={statusCounts}
        />
      </div>

      {/* Compact Upload Zone - Inline dropzone, no card wrapper */}
      <div
        {...getRootProps()}
        className={cn(
          "border-2 border-dashed rounded-lg cursor-pointer transition-all duration-200",
          "flex items-center gap-4 px-4 py-3",
          isDragActive
            ? 'border-primary bg-primary/5'
            : 'border-muted-foreground/20 hover:border-primary/50 hover:bg-muted/30'
        )}
      >
        <input {...getInputProps()} />
        <div className={cn(
          "p-2 rounded-lg transition-all",
          isDragActive ? "bg-primary/10" : "bg-muted/50"
        )}>
          <Upload className={cn(
            "h-5 w-5 transition-all duration-200",
            isDragActive ? "text-primary scale-110" : "text-muted-foreground"
          )} />
        </div>
        <div className="flex-1 min-w-0">
          {isDragActive ? (
            <p className="text-sm font-medium text-primary">Drop files here</p>
          ) : (
            <p className="text-sm text-muted-foreground">
              Drag & drop or <span className="text-primary font-medium">click to upload</span> • TXT, MD, JSON (max 10MB)
            </p>
          )}
        </div>
      </div>
      </div>

      {/* Bulk Actions Bar - Fixed below dropzone */}
      {selectedIds.size > 0 && (
        <div className="shrink-0 px-4 py-2 bg-muted/50 border-b flex items-center justify-between">
          <span className="text-sm font-medium">
            {t('documents.bulk.selected', { count: selectedIds.size }) || `${selectedIds.size} document(s) selected`}
          </span>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={handleBulkReprocess}>
              <RefreshCw className="h-4 w-4 mr-2" />
              {t('documents.bulk.reprocess', 'Reprocess')}
            </Button>
            <Button variant="outline" size="sm" className="text-destructive" onClick={handleBulkDelete}>
              <Trash2 className="h-4 w-4 mr-2" />
              {t('documents.bulk.delete', 'Delete')}
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setSelectedIds(new Set())}>
              <X className="h-4 w-4 mr-2" />
              {t('documents.bulk.clear', 'Clear')}
            </Button>
          </div>
        </div>
      )}

      {/* Upload Progress - Fixed below dropzone when active */}
      {uploadingFiles.length > 0 && (
        <div className="shrink-0 px-4 py-3 border-b space-y-2 bg-muted/20">
          {/* Overall Progress Header */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <h4 className="text-sm font-semibold">
                {isUploading ? (
                  <span className="flex items-center gap-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t('documents.upload.processing', 'Processing Files')}
                  </span>
                ) : (
                  <span className="flex items-center gap-2 text-green-600 dark:text-green-400">
                    <CheckCircle className="h-4 w-4" />
                    {t('documents.upload.complete', 'Upload Complete')}
                  </span>
                )}
              </h4>
            </div>
            <span className="text-xs text-muted-foreground">
              {uploadingFiles.filter(f => f.status === 'success').length}/{uploadingFiles.length} {t('documents.upload.filesComplete', 'files complete')}
            </span>
          </div>
          
          {/* Phase Legend */}
          {isUploading && (
            <div className="flex items-center gap-4 text-xs text-muted-foreground bg-muted/50 rounded-lg px-3 py-2">
              <span className="flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-full bg-amber-500" />
                {t('documents.upload.phase.reading', 'Reading')}
              </span>
              <span className="text-muted-foreground/50">→</span>
              <span className="flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-full bg-blue-500" />
                {t('documents.upload.phase.uploading', 'Uploading')}
              </span>
              <span className="text-muted-foreground/50">→</span>
              <span className="flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-full bg-purple-500" />
                {t('documents.upload.phase.extracting', 'Extracting')}
              </span>
              <span className="text-muted-foreground/50">→</span>
              <span className="flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-full bg-green-500" />
                {t('documents.upload.phase.done', 'Done')}
              </span>
            </div>
          )}
          
          <ScrollArea className="max-h-32">
            <div className="space-y-1">
              {uploadingFiles.map((uploadFile, index) => (
                <div
                  key={`${uploadFile.file.name}-${index}`}
                  className="flex items-center gap-3 p-2 rounded-lg border bg-card"
                >
                  <div className="flex-shrink-0">
                    {uploadFile.status === 'success' ? (
                      <CheckCircle className="h-4 w-4 text-green-500" />
                    ) : uploadFile.status === 'error' ? (
                      <XCircle className="h-4 w-4 text-red-500" />
                    ) : uploadFile.status === 'extracting' ? (
                      <Sparkles className="h-4 w-4 text-purple-500 animate-pulse" />
                    ) : uploadFile.status === 'uploading' ? (
                      <Upload className="h-4 w-4 text-blue-500 animate-bounce" />
                    ) : uploadFile.status === 'reading' ? (
                      <FileSearch className="h-4 w-4 text-amber-500 animate-pulse" />
                    ) : (
                      <Clock className="h-4 w-4 text-muted-foreground" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{uploadFile.file.name}</p>
                    <div className="flex items-center gap-2">
                      <p className="text-xs text-muted-foreground">
                        {(uploadFile.file.size / 1024).toFixed(1)} KB
                      </p>
                      {uploadFile.phase && uploadFile.status !== 'success' && uploadFile.status !== 'error' && (
                        <span className={`text-xs font-medium ${
                          uploadFile.status === 'reading' ? 'text-amber-500' :
                          uploadFile.status === 'uploading' ? 'text-blue-500' :
                          uploadFile.status === 'extracting' ? 'text-purple-500' :
                          'text-muted-foreground'
                        }`}>
                          • {uploadFile.phase}
                        </span>
                      )}
                    </div>
                    {(uploadFile.status === 'reading' || uploadFile.status === 'uploading' || uploadFile.status === 'extracting') && (
                      <Progress value={uploadFile.progress} className="h-1 mt-1" />
                    )}
                    {uploadFile.error && (
                      <p className="text-xs text-red-500 mt-1">{uploadFile.error}</p>
                    )}
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 flex-shrink-0"
                    onClick={() => removeUploadingFile(index)}
                  >
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              ))}
            </div>
          </ScrollArea>
        </div>
      )}

      {/* Batch Progress Card (Phase 2) - Fixed zone when active */}
      {activeTrackId && !isUploading && (
        <div className="shrink-0 px-4 py-3 border-b">
          <BatchProgressCard
            trackId={activeTrackId}
            onClose={() => setActiveTrackId(null)}
            onComplete={() => {
              queryClient.invalidateQueries({ queryKey: ['documents'] });
              setTimeout(() => setActiveTrackId(null), 5000);
            }}
          />
        </div>
      )}

      {/* Scrollable Documents Table Zone */}
      <div className="flex-1 min-h-0 overflow-auto">
        <div className="px-4 py-3">
          {/* Table Header */}
          <div className="flex items-center gap-2 mb-3">
            <FileText className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Documents ({documents.length})</span>
          </div>
          
          {isLoading ? (
            <div className="space-y-2">
              {[...Array(5)].map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : documents.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <FileText className="h-10 w-10 mx-auto mb-3 opacity-50" />
              <p className="font-medium">No documents yet</p>
              <p className="text-sm mt-1">Upload documents to build your knowledge graph</p>
            </div>
          ) : (
            <div className="border rounded-lg overflow-hidden">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[40px]">
                      <Checkbox
                        checked={selectedIds.size === documents.length && documents.length > 0}
                        onCheckedChange={(checked) => handleSelectAll(!!checked)}
                        aria-label={t('documents.bulk.selectAll', 'Select all')}
                      />
                    </TableHead>
                    <TableHead>Title</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Entities</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead className="w-[100px]"></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {documents.map((doc) => (
                    <TableRow 
                      key={doc.id}
                      className={cn(
                        "cursor-pointer hover:bg-muted/50",
                        selectedDocument?.id === doc.id && "bg-muted"
                      )}
                      onClick={() => handleDocumentClick(doc)}
                    >
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <Checkbox
                          checked={selectedIds.has(doc.id)}
                          onCheckedChange={(checked) => handleSelectOne(doc.id, !!checked)}
                          aria-label={t('documents.bulk.select', 'Select')}
                        />
                      </TableCell>
                      <TableCell className="font-medium">
                        {doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`}
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={doc.status || 'completed'} />
                      </TableCell>
                      <TableCell>{doc.entity_count ?? doc.chunk_count ?? '-'}</TableCell>
                      <TableCell className="text-muted-foreground">
                        {doc.created_at 
                          ? formatDistanceToNow(new Date(doc.created_at), { addSuffix: true })
                          : '-'}
                      </TableCell>
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <div className="flex items-center gap-1 justify-end">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => handleDocumentClick(doc)}
                          >
                            <Eye className="h-4 w-4" />
                          </Button>
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" size="icon" className="h-8 w-8">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              {doc.status === 'failed' && (
                                <DropdownMenuItem asChild>
                                  <div className="p-0">
                                    <ResetDocumentStatusButton document={doc} iconOnly={false} size="sm" />
                                  </div>
                                </DropdownMenuItem>
                              )}
                              <DropdownMenuItem onClick={() => reprocessMutation.mutate(doc.id)}>
                                <RefreshCw className="h-4 w-4 mr-2" />
                                {t('documents.actions.reprocess')}
                              </DropdownMenuItem>
                              <DropdownMenuItem
                                onClick={() => deleteMutation.mutate(doc.id)}
                                className="text-destructive"
                              >
                                <Trash2 className="h-4 w-4 mr-2" />
                                {t('documents.actions.delete')}
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      </div>
          
      {/* Fixed Pagination Footer */}
      {documents.length > 0 && (
        <div className="shrink-0 px-4 py-3 border-t bg-background">
          <PaginationControls
            currentPage={currentPage}
            totalPages={totalPages}
            pageSize={pageSize}
            onPageChange={setCurrentPage}
            onPageSizeChange={(newSize) => {
              setPageSize(newSize);
              setCurrentPage(1);
            }}
          />
        </div>
      )}
      </div>

      {/* Right Panel - Document Preview */}
      <RightPanel
        isOpen={previewPanelOpen}
        onToggle={() => setPreviewPanelOpen(!previewPanelOpen)}
        onClose={handlePreviewClose}
        title={selectedDocument ? (selectedDocument.title || selectedDocument.file_name || `Document ${selectedDocument.id.slice(0, 8)}`) : t('documents.preview.title', 'Document Preview')}
        subtitle={selectedDocument?.id ? `ID: ${selectedDocument.id.slice(0, 12)}...` : undefined}
        width="wide"
        showCollapsedBar={true}
        collapsedLabel={t('documents.preview.panelLabel', 'Preview')}
        headerIcon={<FileText className="h-4 w-4" />}
      >
        <DocumentPreviewPanel
          document={selectedDocument}
          onDelete={(id) => {
            deleteMutation.mutate(id);
            handlePreviewClose();
          }}
          onReprocess={(id) => reprocessMutation.mutate(id)}
          onViewFull={(doc) => router.push(`/documents/${doc.id}`)}
          onViewInGraph={handleViewInGraph}
          isDeleting={deleteMutation.isPending}
          isReprocessing={reprocessMutation.isPending}
        />
      </RightPanel>
    </div>
  );
}

export default DocumentManager;
