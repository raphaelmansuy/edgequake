/**
 * @module ErrorBanner
 * @description Component for displaying PDF processing errors with actionable suggestions.
 * Shows error details, affected phase, and retry/cancel actions.
 *
 * @implements OODA-25: Error notification banners
 * @implements UC0712: User sees detailed error messages
 * @implements FEAT0609: Actionable error suggestions
 *
 * @enforces BR0710: Show affected page/chunk in error
 * @enforces BR0711: Provide retry button for recoverable errors
 *
 * @see {@link specs/001-upload-pdf.md} Mission specification
 */
'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleTrigger,
} from '@/components/ui/collapsible';
import {
    AlertCircle,
    ChevronDown,
    ChevronUp,
    FileWarning,
    Loader2,
    RefreshCw,
    X,
    Zap,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

// ============================================================================
// Types
// ============================================================================

/**
 * Error severity levels.
 */
export type ErrorSeverity = 'warning' | 'error' | 'critical';

/**
 * Error context for PDF processing failures.
 */
export interface PdfError {
  /** Error code for classification */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Which phase failed */
  phase?: string;
  /** Page number if applicable */
  page?: number;
  /** Chunk index if applicable */
  chunk?: number;
  /** Stack trace for debugging */
  details?: string;
  /** Whether error is recoverable */
  recoverable?: boolean;
  /** Suggested actions */
  suggestions?: string[];
}

// ============================================================================
// Error Classification
// ============================================================================

/**
 * Classify error by code and return severity + suggestions.
 */
function classifyError(error: PdfError): {
  severity: ErrorSeverity;
  icon: React.ReactNode;
  suggestions: string[];
} {
  const code = error.code?.toLowerCase() || '';

  // Network/timeout errors - usually recoverable
  if (code.includes('timeout') || code.includes('network')) {
    return {
      severity: 'warning',
      icon: <Zap className="h-4 w-4" />,
      suggestions: [
        'Check your internet connection',
        'The file might be too large - try splitting it',
        'Wait a moment and retry',
      ],
    };
  }

  // Rate limiting
  if (code.includes('rate_limit') || code.includes('429')) {
    return {
      severity: 'warning',
      icon: <Zap className="h-4 w-4" />,
      suggestions: [
        'Too many requests - please wait 30 seconds',
        'Consider uploading fewer files at once',
      ],
    };
  }

  // PDF parsing errors
  if (code.includes('parse') || code.includes('corrupt')) {
    return {
      severity: 'error',
      icon: <FileWarning className="h-4 w-4" />,
      suggestions: [
        'The PDF might be corrupted or password-protected',
        'Try re-exporting the PDF from the source application',
        'Convert to PDF/A format for better compatibility',
      ],
    };
  }

  // LLM/extraction errors
  if (code.includes('llm') || code.includes('extraction')) {
    return {
      severity: 'warning',
      icon: <AlertCircle className="h-4 w-4" />,
      suggestions: [
        'The AI model encountered an issue processing this content',
        'Try with a smaller PDF or fewer pages',
        'Retry - this might be a temporary issue',
      ],
    };
  }

  // Storage/database errors
  if (code.includes('storage') || code.includes('database')) {
    return {
      severity: 'critical',
      icon: <AlertCircle className="h-4 w-4" />,
      suggestions: [
        'There was an issue saving to the database',
        'Check server logs for more details',
        'Contact support if the issue persists',
      ],
    };
  }

  // Default fallback
  return {
    severity: 'error',
    icon: <AlertCircle className="h-4 w-4" />,
    suggestions: error.suggestions || [
      'An unexpected error occurred',
      'Try retrying the upload',
      'Contact support if the issue persists',
    ],
  };
}

// ============================================================================
// Component
// ============================================================================

export interface ErrorBannerProps {
  /** The error to display */
  error: PdfError;
  /** Filename for context */
  filename?: string;
  /** Callback when retry is clicked */
  onRetry?: () => void;
  /** Whether retry is in progress */
  isRetrying?: boolean;
  /** Callback when dismiss is clicked */
  onDismiss?: () => void;
  /** Whether to show in compact mode */
  compact?: boolean;
}

export function ErrorBanner({
  error,
  filename,
  onRetry,
  isRetrying = false,
  onDismiss,
  compact = false,
}: ErrorBannerProps) {
  const { t } = useTranslation();
  const [showDetails, setShowDetails] = useState(false);

  const { severity, icon, suggestions } = classifyError(error);

  // Variant mapping for Alert
  const variant = severity === 'warning' ? 'default' : 'destructive';

  // Build context string
  const contextParts: string[] = [];
  if (error.phase) contextParts.push(`Phase: ${error.phase}`);
  if (error.page !== undefined) contextParts.push(`Page: ${error.page}`);
  if (error.chunk !== undefined) contextParts.push(`Chunk: ${error.chunk}`);
  const contextString = contextParts.join(' • ');

  if (compact) {
    return (
      <div className="flex items-center gap-2 p-2 rounded-lg bg-destructive/10 text-destructive text-sm">
        {icon}
        <span className="flex-1 truncate">{error.message}</span>
        {onRetry && error.recoverable !== false && (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2"
            onClick={onRetry}
            disabled={isRetrying}
          >
            {isRetrying ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <RefreshCw className="h-3 w-3" />
            )}
          </Button>
        )}
        {onDismiss && (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2"
            onClick={onDismiss}
          >
            <X className="h-3 w-3" />
          </Button>
        )}
      </div>
    );
  }

  return (
    <Alert variant={variant}>
      <div className="flex items-start gap-3">
        <div className="shrink-0 mt-0.5">{icon}</div>
        <div className="flex-1 space-y-2">
          <AlertTitle className="flex items-center gap-2">
            {filename ? (
              <>
                {t('documents.error.title', 'Failed to process {{name}}', { name: filename })}
              </>
            ) : (
              t('documents.error.titleGeneric', 'Processing failed')
            )}
          </AlertTitle>
          <AlertDescription className="space-y-2">
            <p>{error.message}</p>
            
            {contextString && (
              <p className="text-xs text-muted-foreground font-mono">
                {contextString}
              </p>
            )}

            {/* Suggestions */}
            {suggestions.length > 0 && (
              <div className="mt-2">
                <p className="text-xs font-medium mb-1">
                  {t('documents.error.suggestions', 'Suggestions:')}
                </p>
                <ul className="text-xs space-y-1 list-disc pl-4">
                  {suggestions.map((suggestion, i) => (
                    <li key={i}>{suggestion}</li>
                  ))}
                </ul>
              </div>
            )}

            {/* Details (collapsible) */}
            {error.details && (
              <Collapsible open={showDetails} onOpenChange={setShowDetails}>
                <CollapsibleTrigger asChild>
                  <Button variant="ghost" size="sm" className="h-6 px-2">
                    {showDetails ? (
                      <>
                        <ChevronUp className="h-3 w-3 mr-1" />
                        {t('documents.error.hideDetails', 'Hide details')}
                      </>
                    ) : (
                      <>
                        <ChevronDown className="h-3 w-3 mr-1" />
                        {t('documents.error.showDetails', 'Show details')}
                      </>
                    )}
                  </Button>
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <pre className="mt-2 p-2 text-xs bg-muted rounded overflow-x-auto max-h-32">
                    {error.details}
                  </pre>
                </CollapsibleContent>
              </Collapsible>
            )}

            {/* Actions */}
            <div className="flex items-center gap-2 mt-3">
              {onRetry && error.recoverable !== false && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={onRetry}
                  disabled={isRetrying}
                >
                  {isRetrying ? (
                    <>
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      {t('documents.error.retrying', 'Retrying...')}
                    </>
                  ) : (
                    <>
                      <RefreshCw className="h-4 w-4 mr-2" />
                      {t('documents.error.retry', 'Retry')}
                    </>
                  )}
                </Button>
              )}
              {onDismiss && (
                <Button variant="ghost" size="sm" onClick={onDismiss}>
                  {t('common.dismiss', 'Dismiss')}
                </Button>
              )}
            </div>
          </AlertDescription>
        </div>
      </div>
    </Alert>
  );
}

export default ErrorBanner;
