/**
 * @module ErrorMessagePopover
 * @description Popover component to display error messages with copy-to-clipboard functionality
 *
 * @implements OODA-05 - Enhanced error display for failed documents
 * @implements OODA-09 - Error categorization with actionable suggestions
 * @implements UC0008 - User reprocesses failed documents
 *
 * @enforces BR0302 - Failed documents show clear error information
 */

'use client';

import { Button } from '@/components/ui/button';
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover';
import {
    categorizeError,
    getCategoryColor,
    type CategorizedError,
} from '@/lib/error-categories';
import {
    AlertCircle,
    Brain,
    Check,
    ClipboardCopy,
    Cpu,
    Database,
    FileWarning,
    Lightbulb,
    RefreshCw,
    RotateCcw,
    Wifi,
} from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface ErrorMessagePopoverProps {
  /** The error message to display */
  message: string;
  /** Optional document ID for context */
  documentId?: string;
  /** Optional callback to retry processing */
  onRetry?: () => void;
  /** Whether retry is in progress */
  isRetrying?: boolean;
  /** Additional CSS classes */
  className?: string;
}

/**
 * A popover component that displays error messages with:
 * - Full error text (no truncation)
 * - Copy to clipboard button
 * - Optional retry action
 * - Visual feedback on copy
 *
 * @example
 * ```tsx
 * <ErrorMessagePopover
 *   message="Failed to extract entities: API rate limit"
 *   onRetry={() => handleRetry(doc.id)}
 * />
 * ```
 */
export function ErrorMessagePopover({
  message,
  documentId,
  onRetry,
  isRetrying = false,
  className,
}: ErrorMessagePopoverProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [isOpen, setIsOpen] = useState(false);

  // OODA-09: Categorize the error for user-friendly display
  const categorized: CategorizedError = useMemo(
    () => categorizeError(message),
    [message]
  );
  const colors = useMemo(
    () => getCategoryColor(categorized.category),
    [categorized.category]
  );

  // Get the appropriate icon component for the category
  const CategoryIcon = useMemo(() => {
    switch (categorized.category) {
      case 'llm':
        return Brain;
      case 'embedding':
        return Cpu;
      case 'storage':
        return Database;
      case 'pipeline':
        return FileWarning;
      case 'network':
        return Wifi;
      default:
        return AlertCircle;
    }
  }, [categorized.category]);

  const handleCopy = useCallback(async () => {
    try {
      const textToCopy = documentId
        ? `Document ${documentId}: ${message}`
        : message;

      await navigator.clipboard.writeText(textToCopy);
      setCopied(true);
      toast.success(t('common.copied', 'Copied to clipboard'));

      // Reset copy state after 2 seconds
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [message, documentId, t]);

  const handleRetry = useCallback(() => {
    if (onRetry) {
      onRetry();
      setIsOpen(false);
    }
  }, [onRetry]);

  // Truncate message for trigger display
  const truncatedMessage =
    message.length > 50 ? `${message.slice(0, 47)}...` : message;

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger asChild>
        <button
          type="button"
          className={`text-xs text-red-500 dark:text-red-400 flex items-center gap-1 hover:underline cursor-pointer text-left ${className ?? ''}`}
          data-testid="error-message-trigger"
          onClick={(e) => e.stopPropagation()}
        >
          <AlertCircle className="h-3 w-3 flex-shrink-0" />
          <span className="truncate max-w-[180px]">{truncatedMessage}</span>
        </button>
      </PopoverTrigger>
      <PopoverContent
        className="w-80 p-0"
        align="start"
        onClick={(e) => e.stopPropagation()}
        data-testid="error-message-popover"
      >
        <div className="flex flex-col">
          {/* Header with category */}
          <div className={`flex items-center justify-between gap-2 px-3 py-2 border-b ${colors.bg}`}>
            <div className="flex items-center gap-2">
              <CategoryIcon className={`h-4 w-4 ${colors.text}`} />
              <span className={`text-sm font-medium ${colors.text}`}>
                {t(`documents.error.category.${categorized.category}`, categorized.categoryLabel)}
              </span>
            </div>
            {/* Transient indicator */}
            {categorized.isTransient && (
              <span
                className="flex items-center gap-1 text-xs text-green-600 dark:text-green-400"
                title={t('documents.error.retryable', 'This error may be temporary')}
              >
                <RotateCcw className="h-3 w-3" />
                {t('documents.error.retryable', 'Retryable')}
              </span>
            )}
          </div>

          {/* Error summary */}
          <div className="p-3 space-y-2">
            <p
              className="text-sm font-medium text-foreground"
              data-testid="error-message-summary"
            >
              {categorized.summary}
            </p>
            
            {/* Suggestion */}
            <div className="flex items-start gap-2 p-2 rounded-md bg-muted/50">
              <Lightbulb className="h-4 w-4 text-amber-500 flex-shrink-0 mt-0.5" />
              <p className="text-xs text-muted-foreground">
                {categorized.suggestion}
              </p>
            </div>
          </div>

          {/* Technical details (expandable) */}
          <details className="border-t">
            <summary className="px-3 py-2 text-xs text-muted-foreground cursor-pointer hover:bg-muted/50">
              {t('documents.error.details', 'Technical details')}
            </summary>
            <div className="px-3 pb-3">
              <p
                className="text-xs text-muted-foreground whitespace-pre-wrap break-words font-mono bg-muted/30 p-2 rounded"
                data-testid="error-message-content"
              >
                {message}
              </p>
            </div>
          </details>

          {/* Actions */}
          <div className="flex items-center gap-2 px-3 py-2 border-t bg-muted/50">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopy}
              className="h-7 gap-1.5 text-xs"
              data-testid="error-copy-button"
            >
              {copied ? (
                <>
                  <Check className="h-3.5 w-3.5 text-green-500" />
                  {t('common.copied', 'Copied')}
                </>
              ) : (
                <>
                  <ClipboardCopy className="h-3.5 w-3.5" />
                  {t('common.copy', 'Copy')}
                </>
              )}
            </Button>

            {onRetry && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleRetry}
                disabled={isRetrying}
                className="h-7 gap-1.5 text-xs"
                data-testid="error-retry-button"
              >
                <RefreshCw
                  className={`h-3.5 w-3.5 ${isRetrying ? 'animate-spin' : ''}`}
                />
                {isRetrying
                  ? t('common.retrying', 'Retrying...')
                  : t('common.retry', 'Retry')}
              </Button>
            )}
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}

export default ErrorMessagePopover;
