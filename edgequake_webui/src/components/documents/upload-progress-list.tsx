'use client';

import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
    CheckCircle,
    Clock,
    FileSearch,
    Loader2,
    Sparkles,
    Upload,
    X,
    XCircle,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { PdfUploadProgress } from './pdf-upload-progress';
import type { UploadingFile } from './types';

/**
 * Props for the UploadProgressList component.
 */
interface UploadProgressListProps {
  /** Array of files currently being uploaded */
  uploadingFiles: UploadingFile[];
  /** Whether upload is currently in progress */
  isUploading: boolean;
  /** Callback to remove a file from the upload list */
  onRemove: (index: number) => void;
  /** Callback when a PDF upload completes successfully */
  onComplete: (index: number) => void;
  /** Callback when a PDF upload fails */
  onFailed: (index: number, error: string) => void;
}

/**
 * Displays a list of files being uploaded with their progress and status.
 * 
 * WHY: Extracted from DocumentManager for SRP compliance (OODA-06).
 * This component handles only the visual representation of upload progress.
 * 
 * @implements FEAT0602 - Real-time progress indicators
 */
export function UploadProgressList({
  uploadingFiles,
  isUploading,
  onRemove,
  onComplete,
  onFailed,
}: UploadProgressListProps) {
  const { t } = useTranslation();

  if (uploadingFiles.length === 0) {
    return null;
  }

  return (
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
          {uploadingFiles.filter(f => f.status === 'success').length}/{uploadingFiles.length}{' '}
          {t('documents.upload.filesComplete', 'files complete')}
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
      
      <ScrollArea className="max-h-48">
        <div className="space-y-1">
          {uploadingFiles.map((uploadFile, index) => (
            /* OODA-22: Conditionally render PdfUploadProgress for PDF files with trackId */
            uploadFile.isPdf && uploadFile.trackId ? (
              <div
                key={`${uploadFile.file.name}-${index}`}
                className="relative p-2 rounded-lg border bg-card"
              >
                <PdfUploadProgress
                  trackId={uploadFile.trackId}
                  filename={uploadFile.file.name}
                  compact={true}
                  onComplete={() => onComplete(index)}
                  onFailed={(error) => onFailed(index, error)}
                />
                <Button
                  variant="ghost"
                  size="icon"
                  className="absolute top-1 right-1 h-6 w-6"
                  onClick={() => onRemove(index)}
                >
                  <X className="h-3 w-3" />
                </Button>
              </div>
            ) : (
              <div
                key={`${uploadFile.file.name}-${index}`}
                className="flex items-center gap-3 p-2 rounded-lg border bg-card"
              >
                <div className="shrink-0">
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
                  className="h-6 w-6 shrink-0"
                  onClick={() => onRemove(index)}
                >
                  <X className="h-3 w-3" />
                </Button>
              </div>
            )
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
