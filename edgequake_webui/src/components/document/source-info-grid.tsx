/**
 * @fileoverview Source information grid displaying document metadata
 *
 * @implements FEAT1084 - Source metadata display
 * @implements FEAT1085 - File size formatting
 *
 * @see UC1515 - User views document source info
 * @see UC1516 - User sees processing timestamps
 *
 * @enforces BR1084 - Date formatting with date-fns
 * @enforces BR1085 - Human-readable file sizes
 */
// Source information grid
'use client';

import type { Document } from '@/types';
import { format } from 'date-fns';

interface SourceInfoGridProps {
  document: Document;
}

export function SourceInfoGrid({ document }: SourceInfoGridProps) {
  return (
    <div className="grid gap-y-3 text-sm">
      <InfoRow label="File Name" value={document.file_name || 'N/A'} />
      <InfoRow label="MIME Type" value={document.mime_type || 'Unknown'} mono />
      <InfoRow label="Source Type" value={document.source_type || 'Unknown'} className="capitalize" />
      {document.document_type && (
        <InfoRow label="Document Type" value={document.document_type} className="capitalize" />
      )}
      <InfoRow 
        label="Content Length" 
        value={document.content_length ? `${document.content_length.toLocaleString()} chars` : '-'} 
      />
      <InfoRow 
        label="File Size" 
        value={document.file_size ? formatFileSize(document.file_size) : 
               document.file_size_bytes ? formatFileSize(document.file_size_bytes) : '-'} 
      />
      {document.page_count && document.page_count > 0 && (
        <InfoRow label="Pages" value={`${document.page_count}`} />
      )}
      {document.sha256_checksum && (
        <InfoRow label="SHA-256" value={document.sha256_checksum.slice(0, 16) + '...'} mono />
      )}
      {document.created_at && (
        <InfoRow 
          label="Created At" 
          value={format(new Date(document.created_at), 'PPpp')} 
        />
      )}
      {document.processed_at && (
        <InfoRow 
          label="Processed At" 
          value={format(new Date(document.processed_at), 'PPpp')} 
        />
      )}
      {document.track_id && (
        <InfoRow label="Track ID" value={document.track_id} mono />
      )}
    </div>
  );
}

interface InfoRowProps {
  label: string;
  value: string;
  mono?: boolean;
  className?: string;
}

function InfoRow({ label, value, mono, className }: InfoRowProps) {
  return (
    <div>
      <span className="text-muted-foreground block text-xs mb-0.5">{label}</span>
      <p className={`font-medium truncate ${mono ? 'font-mono text-xs' : ''} ${className || ''}`}>
        {value}
      </p>
    </div>
  );
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}
