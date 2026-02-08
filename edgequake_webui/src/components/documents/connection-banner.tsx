/**
 * @module ConnectionBanner
 * @description Persistent banner shown when WebSocket connection is lost.
 * OODA-02: Fix silent WebSocket disconnection.
 *
 * @implements FEAT0867 - Connection state visibility
 * @implements UC0714 - User sees when real-time updates unavailable
 *
 * @enforces BR0867 - User aware of connection status
 * @enforces BR0868 - Manual retry available when disconnected
 */
'use client';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { useWebSocket } from '@/hooks/use-websocket';
import { useIngestionStore } from '@/stores/use-ingestion-store';
import { AlertCircle, RefreshCw, X } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

// ============================================================================
// Component
// ============================================================================

export interface ConnectionBannerProps {
  /** Additional CSS classes */
  className?: string;
}

/**
 * Persistent banner displayed when WebSocket connection is lost.
 * Shows only when max reconnect attempts have been reached.
 * Provides retry and dismiss actions.
 */
export function ConnectionBanner({ className }: ConnectionBannerProps) {
  const { t } = useTranslation();
  const { connect } = useWebSocket();
  const wsMaxReconnectsReached = useIngestionStore((s) => s.wsMaxReconnectsReached);
  const setWsMaxReconnectsReached = useIngestionStore((s) => s.setWsMaxReconnectsReached);
  const [dismissed, setDismissed] = useState(false);

  // Only show if max reconnects reached and not dismissed
  if (!wsMaxReconnectsReached || dismissed) {
    return null;
  }

  const handleRetry = () => {
    setWsMaxReconnectsReached(false);
    connect();
  };

  const handleDismiss = () => {
    setDismissed(true);
  };

  return (
    <Alert variant="destructive" className={className}>
      <AlertCircle className="h-4 w-4" />
      <AlertTitle>
        {t('connection.banner.title', 'Connection Lost')}
      </AlertTitle>
      <AlertDescription className="flex items-center justify-between gap-4">
        <span>
          {t(
            'connection.banner.description',
            'Real-time updates are unavailable. Document progress may be delayed.'
          )}
        </span>
        <div className="flex items-center gap-2 shrink-0">
          <Button variant="outline" size="sm" onClick={handleRetry}>
            <RefreshCw className="h-4 w-4 mr-1" />
            {t('connection.banner.retry', 'Retry')}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6"
            onClick={handleDismiss}
            aria-label={t('connection.banner.dismiss', 'Dismiss')}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </AlertDescription>
    </Alert>
  );
}

export default ConnectionBanner;
