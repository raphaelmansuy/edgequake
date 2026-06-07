'use client';

import { cn } from '@/lib/utils';
import { Upload } from 'lucide-react';
import type React from 'react';
import type { DropzoneInputProps, DropzoneRootProps } from 'react-dropzone';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useTranslation } from 'react-i18next';

/**
 * Props for the DocumentDropzone component.
 */
export interface DocumentDropzoneProps {
  /** Props to spread on the dropzone container */
  getRootProps: <T extends DropzoneRootProps>(props?: T) => T;
  /** Props to spread on the hidden file input */
  getInputProps: <T extends DropzoneInputProps>(props?: T) => T;
  /** Whether a drag operation is currently active over the zone */
  isDragActive: boolean;
  /** Function to programmatically open file dialog (explicit click handler) */
  openFileDialog: () => void;
  /** Per-upload PDF parser backend override. */
  pdfParserBackend: 'default' | 'vision' | 'edgeparse';
  /** Change handler for the PDF parser override selector. */
  onPdfParserBackendChange: (value: 'default' | 'vision' | 'edgeparse') => void;
}

/**
 * Compact file upload dropzone with drag-and-drop support.
 * 
 * WHY: Extracted from DocumentManager for SRP compliance (OODA-08).
 * This component handles only the visual presentation of the dropzone.
 * 
 * WHY explicit onClick: react-dropzone's internal click handler (noClick: false)
 * can silently fail with the File System Access API in certain browsers/contexts.
 * We disable noClick and use an explicit onClick → openFileDialog() for reliable
 * cross-browser file dialog opening. See:
 * - https://github.com/react-dropzone/react-dropzone/issues/1127
 * - https://github.com/react-dropzone/react-dropzone/issues/1349
 * 
 * @implements FEAT0001 - Document ingestion with entity extraction
 */
export function DocumentDropzone({
  getRootProps,
  getInputProps,
  isDragActive,
  openFileDialog,
  pdfParserBackend,
  onPdfParserBackendChange,
}: DocumentDropzoneProps) {
  const { t } = useTranslation();
  return (
    <div
      {...getRootProps({
        onClick: (e: React.MouseEvent) => {
          e.stopPropagation();
          openFileDialog();
        },
        role: 'button' as const,
        'aria-label': t('documents.upload.uploadDrop', 'Upload files by clicking or dragging'),
        tabIndex: 0,
      })}
      className={cn(
        "border-2 border-dashed rounded-lg cursor-pointer transition-all duration-200",
        "flex items-center gap-4 px-4 py-3",
        isDragActive
          ? 'border-primary bg-primary/5 ring-2 ring-primary/20 animate-pulse'
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
          <p className="text-sm font-medium text-primary">{t('documents.upload.uploadDropActive', 'Drop files here')}</p>
        ) : (
          <div className="space-y-1">
            <p className="text-sm text-muted-foreground">
              {t('documents.upload.uploadDrop', 'Drag & drop or click to upload')} • TXT, MD, JSON, PDF, PNG, JPG, GIF, WEBP (max 100MB)
            </p>
            <p className="text-xs text-muted-foreground">
              {t(
                'documents.upload.pdfParserHint',
                'Choose a PDF parser override for this upload, or keep the workspace default.',
              )}
            </p>
          </div>
        )}
      </div>
      <div
        className="flex items-center gap-2"
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.stopPropagation()}
      >
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {t('documents.upload.pdfParser', 'Parser for this upload')}
        </span>
        <Select
          value={pdfParserBackend}
          onValueChange={(value: 'default' | 'vision' | 'edgeparse') =>
            onPdfParserBackendChange(value)
          }
        >
          <SelectTrigger className="w-[190px] h-9 bg-background">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="default">
              {t('documents.upload.pdfParserDefault', 'Workspace Default')}
            </SelectItem>
            <SelectItem value="vision">
              {t('documents.upload.pdfParserVision', 'Vision')}
            </SelectItem>
            <SelectItem value="edgeparse">
              {t('documents.upload.pdfParserEdgeParse', 'EdgeParse')}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
