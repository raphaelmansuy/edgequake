'use client';

import { cn } from '@/lib/utils';
import { Upload } from 'lucide-react';
import type { DropzoneInputProps, DropzoneRootProps } from 'react-dropzone';

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
}

/**
 * Compact file upload dropzone with drag-and-drop support.
 * 
 * WHY: Extracted from DocumentManager for SRP compliance (OODA-08).
 * This component handles only the visual presentation of the dropzone.
 * 
 * @implements FEAT0001 - Document ingestion with entity extraction
 */
export function DocumentDropzone({
  getRootProps,
  getInputProps,
  isDragActive,
}: DocumentDropzoneProps) {
  return (
    <div
      {...getRootProps()}
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
          <p className="text-sm font-medium text-primary">Drop files here</p>
        ) : (
          <p className="text-sm text-muted-foreground">
            Drag & drop or <span className="text-primary font-medium">click to upload</span> • TXT, MD, JSON, PDF (max 10MB)
          </p>
        )}
      </div>
    </div>
  );
}
