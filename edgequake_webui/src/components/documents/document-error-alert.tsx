/**
 * @module DocumentErrorAlert
 * @description Error alert for document loading failures.
 * Extracted from DocumentManager for SRP compliance (OODA-25).
 * 
 * @implements FEAT0601 - Error display
 */
'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { AlertCircle } from 'lucide-react';

/**
 * Props for DocumentErrorAlert component.
 */
export interface DocumentErrorAlertProps {
  /** Error object or message */
  error: Error | unknown;
  /** Handler to retry loading */
  onRetry: () => void;
}

/**
 * Error alert displayed when document loading fails.
 */
export function DocumentErrorAlert({ error, onRetry }: DocumentErrorAlertProps) {
  return (
    <div className="p-6">
      <Alert variant="destructive">
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>Error loading documents</AlertTitle>
        <AlertDescription>
          {error instanceof Error ? error.message : 'Failed to load documents'}
          <Button variant="link" className="ml-2 p-0" onClick={onRetry}>
            Try again
          </Button>
        </AlertDescription>
      </Alert>
    </div>
  );
}

export default DocumentErrorAlert;
