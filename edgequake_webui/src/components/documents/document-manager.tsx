/**
 * @module DocumentManager
 * @description Document ingestion and management interface.
 * Supports file upload, progress tracking, status monitoring, and batch operations.
 * 
 * @implements UC0001 - User uploads documents for ingestion
 * @implements UC0007 - User monitors document processing progress
 * @implements UC0008 - User reprocesses failed documents
 * @implements UC0009 - User deletes documents from knowledge graph
 * @implements FEAT0001 - Document ingestion with entity extraction
 * @implements FEAT0003 - Batch document processing
 * @implements FEAT0004 - Processing status tracking per document
 * @implements FEAT0602 - Real-time progress indicators
 * 
 * @enforces BR0302 - Failed documents can be reprocessed
 * @enforces BR0303 - Document deletion cascades to related entities
 * @enforces BR0305 - Cost tracking per document ingestion
 * 
 * @see {@link docs/use_cases.md} UC0001, UC0007-UC0009
 * @see {@link docs/features.md} FEAT0001, FEAT0003
 */
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
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import {
    cancelTask,
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
    Copy,
    Eye,
    File,
    FileCode,
    FileImage,
    FileSearch,
    FileSpreadsheet,
    FileText,
    FileType,
    Loader2,
    MoreVertical,
    RefreshCw,
    Search,
    Sparkles,
    StopCircle,
    Trash2,
    Upload,
    X,
    XCircle,
} from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { BatchProgressCard } from './batch-progress-card';
import { ClearDocumentsDialog } from './clear-documents-dialog';
import { CostCell } from './cost-cell';
import { DocumentFilters, type DocStatus, type SortDirection, type SortField } from './document-filters';
import { DocumentPreviewPanel } from './document-preview-panel';
import { ErrorMessagePopover } from './error-message-popover';
import { PaginationControls } from './pagination-controls';
import { PipelineStatusDialog } from './pipeline-status-dialog';
import { ReprocessFailedButton } from './reprocess-failed-button';
import { ResetDocumentStatusButton } from './reset-document-status-button';
import { StatusBadge, type DocumentStatus } from './status-badge';
import type { UploadingFile } from './types';

/**
 * OODA-30: File type icon helper
 * WHY: Visual distinction helps users quickly identify document types
 */
function getFileTypeIcon(fileName: string | undefined | null) {
  if (!fileName) return { icon: File, color: 'text-muted-foreground' };
  const ext = fileName.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'pdf':
      return { icon: FileText, color: 'text-red-500' };
    case 'doc':
    case 'docx':
      return { icon: FileType, color: 'text-blue-500' };
    case 'xls':
    case 'xlsx':
    case 'csv':
      return { icon: FileSpreadsheet, color: 'text-green-500' };
    case 'md':
    case 'markdown':
      return { icon: FileCode, color: 'text-purple-500' };
    case 'txt':
      return { icon: FileText, color: 'text-gray-500' };
    case 'html':
    case 'htm':
    case 'json':
    case 'xml':
      return { icon: FileCode, color: 'text-orange-500' };
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'webp':
      return { icon: FileImage, color: 'text-pink-500' };
    default:
      return { icon: File, color: 'text-muted-foreground' };
  }
}

/**
 * OODA-32: Highlight search matches in text
 * WHY: Visual feedback shows which part of title matched the search
 */
function highlightMatches(text: string, query: string): React.ReactNode {
  if (!query.trim()) return text;
  const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
  const parts = text.split(regex);
  return parts.map((part, i) =>
    regex.test(part) ? (
      <mark key={i} className="bg-yellow-200 dark:bg-yellow-700 px-0.5 rounded">
        {part}
      </mark>
    ) : (
      part
    )
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
  // OODA-29: Initialize pageSize from localStorage for persistence
  const [pageSize, setPageSize] = useState(() => {
    if (typeof window === 'undefined') return 20;
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      const size = parsed?.pageSize;
      return [10, 20, 50, 100].includes(size) ? size : 20;
    } catch { return 20; }
  });
  
  // Filter and sort state
  // OODA-24/28: Initialize from localStorage for persistence
  const [statusFilter, setStatusFilter] = useState<DocStatus>(() => {
    if (typeof window === 'undefined') return 'all';
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      return (parsed?.statusFilter as DocStatus) || 'all';
    } catch { return 'all'; }
  });
  const [sortField, setSortField] = useState<SortField>(() => {
    if (typeof window === 'undefined') return 'created_at';
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      return (parsed?.sortField as SortField) || 'created_at';
    } catch { return 'created_at'; }
  });
  const [sortDirection, setSortDirection] = useState<SortDirection>(() => {
    if (typeof window === 'undefined') return 'desc';
    try {
      const stored = localStorage.getItem('edgequake:documents:prefs');
      const parsed = stored ? JSON.parse(stored) : null;
      return (parsed?.sortDirection as SortDirection) || 'desc';
    } catch { return 'desc'; }
  });
  
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

  // Cancel mutation for stopping document processing
  const cancelMutation = useMutation({
    mutationFn: async (trackId: string) => {
      await cancelTask(trackId);
    },
    onSuccess: () => {
      toast.success(t('documents.cancel.success', 'Document processing cancelled'), {
        duration: 4000,
        description: t('documents.cancel.successDesc', 'The extraction has been stopped.'),
      });
      queryClient.invalidateQueries({ queryKey: ['documents'] });
    },
    onError: (error) => {
      toast.error(t('documents.cancel.failed', 'Cancel failed'), {
        description: error instanceof Error ? error.message : t('documents.cancel.failedDesc', 'Could not cancel processing. It may have already completed.'),
      });
    },
  });

  const { getRootProps, getInputProps, isDragActive, open: openFileDialog } = useDropzone({
    onDrop,
    accept: {
      'text/plain': ['.txt'],
      'text/markdown': ['.md'],
      'application/json': ['.json'],
    },
    maxSize: MAX_FILE_SIZE, // 10MB limit
    noClick: false, // Allow click on dropzone
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
    cancelled: serverStatusCounts.cancelled || 0,
  } : {
    all: allDocuments.length,
    pending: allDocuments.filter((d) => d.status === 'pending').length,
    processing: allDocuments.filter((d) => d.status === 'processing').length,
    completed: allDocuments.filter((d) => !d.status || d.status === 'completed' || d.status === 'indexed').length,
    failed: allDocuments.filter((d) => d.status === 'failed').length,
    cancelled: allDocuments.filter((d) => d.status === 'cancelled').length,
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

    // Get documents to find their track_ids
    const documents = data?.items || [];
    
    for (const id of idsToReprocess) {
      try {
        const doc = documents.find(d => d.id === id);
        if (!doc?.track_id) {
          errorCount++;
          continue;
        }
        await reprocessDocument(doc.track_id);
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
  }, [selectedIds, data, queryClient, t]);

  // Document selection for preview panel
  const handleDocumentClick = useCallback((doc: Document) => {
    setSelectedDocument(doc);
    setPreviewPanelOpen(true);
  }, []);

  /**
   * OODA-40: Double-click to navigate to graph
   * WHY: Power users expect double-click for primary navigation action
   */
  const handleDocumentDoubleClick = useCallback((doc: Document) => {
    if (doc.status === 'completed') {
      router.push(`/graph?entity=${encodeURIComponent(doc.id)}`);
    }
  }, [router]);

  const handlePreviewClose = useCallback(() => {
    setSelectedDocument(null);
    setPreviewPanelOpen(false);
  }, []);

  const handleViewInGraph = useCallback((doc: Document) => {
    router.push(`/graph?entity=${encodeURIComponent(doc.id)}`);
  }, [router]);

  /**
   * OODA-19: Keyboard shortcuts for power users
   * WHY: Keyboard shortcuts improve efficiency and accessibility
   * 
   * Shortcuts:
   * - Escape: Clear selection or close preview panel
   * - Ctrl/Cmd + A: Select all documents
   * - R: Refresh document list (when not in input)
   */
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if in input field or textarea
      const target = e.target as HTMLElement;
      const tagName = target.tagName.toUpperCase();
      if (tagName === 'INPUT' || tagName === 'TEXTAREA' || target.isContentEditable) {
        return;
      }

      // Escape: Clear selection or close preview panel
      if (e.key === 'Escape') {
        if (previewPanelOpen) {
          handlePreviewClose();
        } else if (selectedIds.size > 0) {
          setSelectedIds(new Set());
        }
        return;
      }

      // Ctrl/Cmd + A: Select all documents
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'a') {
        e.preventDefault(); // Prevent browser select all
        handleSelectAll(true);
        return;
      }

      // R: Refresh documents (single key, no modifier)
      if (e.key.toLowerCase() === 'r' && !e.metaKey && !e.ctrlKey && !e.altKey) {
        refetch();
        toast.info(t('documents.refresh.triggered', 'Refreshing documents...'), { duration: 1000 });
        return;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [previewPanelOpen, selectedIds.size, handlePreviewClose, handleSelectAll, refetch, t]);

  /**
   * OODA-24: Persist sort preferences to localStorage
   * WHY: Users expect their filter/sort preferences to persist across sessions
   */
  useEffect(() => {
    try {
      localStorage.setItem('edgequake:documents:prefs', JSON.stringify({
        statusFilter,
        sortField,
        sortDirection,
        pageSize, // OODA-29: Also persist page size preference
      }));
    } catch {
      // Ignore localStorage errors (e.g., in incognito mode)
    }
  }, [statusFilter, sortField, sortDirection, pageSize]);

  /**
   * OODA-26: Update page title with document count
   * WHY: Users can see document count without switching tabs
   */
  useEffect(() => {
    const baseTitle = 'Documents - EdgeQuake';
    const count = totalCount || 0;
    const processing = pipelineStatus?.running_tasks || 0;
    
    if (processing > 0) {
      document.title = `⏳ Processing (${processing}) | Documents (${count}) - EdgeQuake`;
    } else if (count > 0) {
      document.title = `Documents (${count}) - EdgeQuake`;
    } else {
      document.title = baseTitle;
    }
    
    return () => { document.title = baseTitle; };
  }, [totalCount, pipelineStatus?.running_tasks]);

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
              <div className="flex items-center gap-2">
                <h1 className="text-xl font-semibold tracking-tight">{t('documents.title')}</h1>
                {/* OODA-39: Document count badge */}
                {totalCount > 0 && (
                  <Badge variant="secondary" className="text-xs font-normal">
                    {totalCount}
                  </Badge>
                )}
              </div>
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
            className="pl-9 pr-8 h-9"
          />
          {/* OODA-36: Clear search button */}
          {searchQuery && (
            <button
              type="button"
              onClick={() => setSearchQuery('')}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-muted transition-colors"
              aria-label="Clear search"
            >
              <X className="h-3.5 w-3.5 text-muted-foreground" />
            </button>
          )}
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

      {/* OODA-23: Compact Processing Status Summary */}
      {pipelineStatus && (pipelineStatus.running_tasks > 0 || pipelineStatus.queued_tasks > 0) && (
        <div 
          className="flex items-center gap-4 px-3 py-2 bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 rounded-lg cursor-pointer hover:bg-blue-100 dark:hover:bg-blue-950/50 transition-colors"
          onClick={() => setPipelineDialogOpen(true)}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => e.key === 'Enter' && setPipelineDialogOpen(true)}
        >
          <Loader2 className="h-4 w-4 text-blue-600 dark:text-blue-400 animate-spin shrink-0" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-blue-700 dark:text-blue-300">
              {pipelineStatus.running_tasks > 0 
                ? t('pipeline.processing', 'Processing {{count}} document(s)', { count: pipelineStatus.running_tasks })
                : t('pipeline.queued', '{{count}} document(s) queued', { count: pipelineStatus.queued_tasks })
              }
            </p>
          </div>
          <div className="flex items-center gap-3 text-xs text-blue-600 dark:text-blue-400">
            {pipelineStatus.queued_tasks > 0 && pipelineStatus.running_tasks > 0 && (
              <span className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {pipelineStatus.queued_tasks} queued
              </span>
            )}
            {pipelineStatus.completed_tasks > 0 && (
              <span className="flex items-center gap-1">
                <CheckCircle className="h-3 w-3 text-green-600" />
                {pipelineStatus.completed_tasks} done
              </span>
            )}
            <span className="text-blue-500">Click for details →</span>
          </div>
        </div>
      )}

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
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium">
              {t('documents.bulk.selected', { count: selectedIds.size }) || `${selectedIds.size} document(s) selected`}
            </span>
            {/* OODA-19: Keyboard hint */}
            <span className="text-xs text-muted-foreground hidden sm:inline">
              Press <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">Esc</kbd> to clear
            </span>
          </div>
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
            /* OODA-20: Enhanced loading skeleton matching table structure */
            <div className="border rounded-lg overflow-hidden">
              {[...Array(5)].map((_, i) => (
                <div key={i} className="flex items-center gap-4 px-4 py-3 border-b last:border-b-0 animate-pulse">
                  <Skeleton className="h-4 w-4 shrink-0 rounded" />
                  <Skeleton className="h-4 w-48 shrink-0" />
                  <Skeleton className="h-5 w-20 rounded-full shrink-0" />
                  <Skeleton className="h-4 w-8 shrink-0" />
                  <Skeleton className="h-4 w-12 shrink-0" />
                  <Skeleton className="h-4 w-24 shrink-0" />
                  <Skeleton className="h-6 w-6 rounded-full shrink-0 ml-auto" />
                </div>
              ))}
            </div>
          ) : documents.length === 0 ? (
            /* OODA-20: Enhanced empty state with upload CTA */
            <div className="text-center py-16 text-muted-foreground border rounded-lg bg-muted/5">
              <FileText className="h-12 w-12 mx-auto mb-4 opacity-40" />
              <p className="font-medium text-lg text-foreground">No documents yet</p>
              <p className="text-sm mt-2 max-w-sm mx-auto">
                Drag & drop files above or click to upload. Build your knowledge graph from documents.
              </p>
              <Button 
                variant="outline" 
                className="mt-4"
                onClick={openFileDialog}
              >
                <Upload className="h-4 w-4 mr-2" />
                Upload Documents
              </Button>
            </div>
          ) : (
            <div className="border rounded-lg overflow-hidden shadow-sm">
              <Table>
                <TableHeader className="bg-muted/50 sticky top-0 z-10">
                  <TableRow className="hover:bg-transparent">
                    <TableHead className="w-[40px]">
                      <Checkbox
                        checked={selectedIds.size === documents.length && documents.length > 0}
                        onCheckedChange={(checked) => handleSelectAll(!!checked)}
                        aria-label={t('documents.bulk.selectAll', 'Select all')}
                      />
                    </TableHead>
                    <TableHead>Title</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="text-center">Entities</TableHead>
                    <TableHead className="text-center">Cost</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead className="w-[100px]"></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {documents.map((doc, index) => (
                    <TableRow 
                      key={doc.id}
                      className={cn(
                        "cursor-pointer transition-colors duration-150",
                        "hover:bg-primary/5 dark:hover:bg-primary/10",
                        selectedDocument?.id === doc.id && "bg-primary/10 dark:bg-primary/15 ring-1 ring-primary/20",
                        index % 2 === 0 ? "bg-background" : "bg-muted/20",
                        // OODA-25: Failed documents highlight
                        doc.status === 'failed' && "bg-red-50/50 dark:bg-red-950/20 border-l-4 border-l-red-500"
                      )}
                      onClick={() => handleDocumentClick(doc)}
                      onDoubleClick={() => handleDocumentDoubleClick(doc)}
                    >
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <Checkbox
                          checked={selectedIds.has(doc.id)}
                          onCheckedChange={(checked) => handleSelectOne(doc.id, !!checked)}
                          aria-label={t('documents.bulk.select', 'Select')}
                        />
                      </TableCell>
                      <TableCell className="font-medium">
                        <div className="flex flex-col gap-0.5">
                          {/* OODA-30: File type icon for visual identification */}
                          <div className="flex items-center gap-2">
                            {(() => {
                              const { icon: FileIcon, color } = getFileTypeIcon(doc.file_name);
                              return <FileIcon className={cn("h-4 w-4 shrink-0", color)} />;
                            })()}
                            {/* OODA-32: Highlight search matches */}
                            <span className="truncate">
                              {highlightMatches(
                                doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`,
                                searchQuery
                              )}
                            </span>
                          </div>
                          {/* OODA-05: Enhanced error display with copy and retry */}
                          {doc.status === 'failed' && doc.error_message && (
                            <ErrorMessagePopover
                              message={doc.error_message}
                              documentId={doc.id}
                              onRetry={() => reprocessMutation.mutate(doc.id)}
                              isRetrying={reprocessMutation.isPending}
                            />
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={doc.status || 'completed'} />
                      </TableCell>
                      <TableCell className="text-center">{doc.entity_count ?? doc.chunk_count ?? '-'}</TableCell>
                      <TableCell className="text-center">
                        <CostCell 
                          document={doc}
                          size="sm" 
                        />
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {doc.created_at 
                          ? (
                            <div className="flex items-center gap-1.5">
                              {/* OODA-34: "New" indicator for documents created within 1 hour */}
                              {new Date().getTime() - new Date(doc.created_at).getTime() < 3600000 && (
                                <span className="text-xs font-medium text-green-600 dark:text-green-400 animate-pulse">
                                  NEW
                                </span>
                              )}
                              <span>{formatDistanceToNow(new Date(doc.created_at), { addSuffix: true })}</span>
                            </div>
                          )
                          : '-'}
                      </TableCell>
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <div className="flex items-center gap-1 justify-end">
                          {/* OODA-22: Quick action buttons */}
                          
                          {/* Preview button (always visible) */}
                          <TooltipProvider delayDuration={300}>
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  className="h-8 w-8"
                                  onClick={() => handleDocumentClick(doc)}
                                >
                                  <Eye className="h-4 w-4" />
                                </Button>
                              </TooltipTrigger>
                              <TooltipContent>Preview</TooltipContent>
                            </Tooltip>
                          </TooltipProvider>
                          
                          {/* View in Graph button (for completed documents) */}
                          {(doc.status === 'completed' || doc.status === 'indexed') && (
                            <TooltipProvider delayDuration={300}>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-8 w-8"
                                    onClick={() => handleViewInGraph(doc)}
                                  >
                                    <Sparkles className="h-4 w-4" />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>View in Graph</TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          )}
                          
                          {/* Retry button (for failed documents) */}
                          {doc.status === 'failed' && (
                            <TooltipProvider delayDuration={300}>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="icon"
                                    className="h-8 w-8 text-orange-600 hover:text-orange-700 hover:bg-orange-50"
                                    onClick={() => reprocessMutation.mutate(doc.id)}
                                  >
                                    <RefreshCw className={`h-4 w-4 ${reprocessMutation.isPending ? 'animate-spin' : ''}`} />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>Retry</TooltipContent>
                              </Tooltip>
                            </TooltipProvider>
                          )}
                          
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" size="icon" className="h-8 w-8">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              {/* OODA-31: Copy document ID */}
                              <DropdownMenuItem 
                                onClick={() => {
                                  navigator.clipboard.writeText(doc.id);
                                  toast.success(t('documents.actions.idCopied', 'Document ID copied'));
                                }}
                              >
                                <Copy className="h-4 w-4 mr-2" />
                                {t('documents.actions.copyId', 'Copy ID')}
                              </DropdownMenuItem>
                              {doc.status === 'failed' && (
                                <DropdownMenuItem asChild>
                                  <div className="p-0">
                                    <ResetDocumentStatusButton document={doc} iconOnly={false} size="sm" />
                                  </div>
                                </DropdownMenuItem>
                              )}
                              {/* Cancel option for pending/processing documents */}
                              {(doc.status === 'pending' || doc.status === 'processing') && doc.track_id && (
                                <DropdownMenuItem 
                                  onClick={() => cancelMutation.mutate(doc.track_id!)}
                                  className="text-orange-600"
                                >
                                  <StopCircle className="h-4 w-4 mr-2" />
                                  {t('documents.actions.cancel', 'Cancel Extraction')}
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
          {/* OODA-37: Show filtered vs total count when filtering */}
          {(searchQuery || statusFilter !== 'all') && (
            <p className="text-xs text-muted-foreground mb-2 text-center">
              Showing {documents.length} of {totalCount} documents
              {statusFilter !== 'all' && ` (${statusFilter})`}
              {searchQuery && ` matching "${searchQuery}"`}
            </p>
          )}
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
