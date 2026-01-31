/**
 * @module ConnectionStatus
 * @description Component for displaying WebSocket connection status.
 * Shows visual indicator for connected, disconnected, or reconnecting states.
 *
 * @implements OODA-27: Connection status indicator
 * @implements UC0713: User sees real-time connection status
 * @implements FEAT0610: Visual connection state feedback
 *
 * @enforces BR0604: Auto-reconnect visible to user
 * @enforces BR0605: Connection state syncs across components
 *
 * @see {@link specs/001-upload-pdf.md} Mission specification
 */
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { useWebSocket } from '@/hooks/use-websocket';
import { cn } from '@/lib/utils';
import { Loader2, Wifi, WifiOff, Zap } from 'lucide-react';
import { useTranslation } from 'react-i18next';

// ============================================================================
// Types
// ============================================================================

export type ConnectionState = 'connected' | 'disconnected' | 'reconnecting';

// ============================================================================
// Component
// ============================================================================

export interface ConnectionStatusProps {
  /** Whether to show in compact mode (just icon) */
  compact?: boolean;
  /** Additional CSS classes */
  className?: string;
  /** Whether to show connect/disconnect buttons */
  showActions?: boolean;
}

export function ConnectionStatus({
  compact = false,
  className,
  showActions = false,
}: ConnectionStatusProps) {
  const { t } = useTranslation();
  const { connected, reconnecting, connect, disconnect } = useWebSocket();

  // Determine current state
  const state: ConnectionState = reconnecting
    ? 'reconnecting'
    : connected
      ? 'connected'
      : 'disconnected';

  // State-specific styling and content
  const stateConfig = {
    connected: {
      icon: <Wifi className="h-3 w-3" />,
      label: t('connection.status.connected', 'Live'),
      description: t('connection.status.connectedDesc', 'Real-time updates active'),
      color: 'text-green-500',
      bgColor: 'bg-green-500',
      pulseColor: 'bg-green-400',
    },
    disconnected: {
      icon: <WifiOff className="h-3 w-3" />,
      label: t('connection.status.disconnected', 'Offline'),
      description: t('connection.status.disconnectedDesc', 'Using polling for updates'),
      color: 'text-muted-foreground',
      bgColor: 'bg-muted-foreground',
      pulseColor: 'bg-muted-foreground',
    },
    reconnecting: {
      icon: <Loader2 className="h-3 w-3 animate-spin" />,
      label: t('connection.status.reconnecting', 'Connecting...'),
      description: t('connection.status.reconnectingDesc', 'Attempting to reconnect'),
      color: 'text-amber-500',
      bgColor: 'bg-amber-500',
      pulseColor: 'bg-amber-400',
    },
  };

  const config = stateConfig[state];

  // Compact mode - just a pulsing dot
  if (compact) {
    return (
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className={cn('relative flex items-center', className)}>
              {/* Pulse animation for connected state */}
              {state === 'connected' && (
                <span className="absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75 animate-ping" />
              )}
              <span
                className={cn(
                  'relative inline-flex h-2 w-2 rounded-full',
                  config.bgColor
                )}
              />
            </div>
          </TooltipTrigger>
          <TooltipContent>
            <div className="flex items-center gap-2">
              {config.icon}
              <span>{config.description}</span>
            </div>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    );
  }

  // Full mode with badge and actions
  return (
    <div className={cn('flex items-center gap-2', className)}>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Badge
              variant="outline"
              className={cn(
                'gap-1.5 cursor-default',
                state === 'connected' && 'border-green-500/50 bg-green-500/10',
                state === 'disconnected' && 'border-muted-foreground/50',
                state === 'reconnecting' && 'border-amber-500/50 bg-amber-500/10'
              )}
            >
              {/* Status dot */}
              <span className="relative flex h-2 w-2">
                {state === 'connected' && (
                  <span className="absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75 animate-ping" />
                )}
                <span
                  className={cn(
                    'relative inline-flex h-2 w-2 rounded-full',
                    config.bgColor
                  )}
                />
              </span>
              
              <span className={cn('text-xs font-medium', config.color)}>
                {config.label}
              </span>
            </Badge>
          </TooltipTrigger>
          <TooltipContent>
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2">
                {config.icon}
                <span className="font-medium">{config.description}</span>
              </div>
              {state === 'connected' && (
                <span className="text-xs text-muted-foreground flex items-center gap-1">
                  <Zap className="h-3 w-3" />
                  Updates in &lt;500ms
                </span>
              )}
              {state === 'disconnected' && (
                <span className="text-xs text-muted-foreground">
                  Click to reconnect
                </span>
              )}
            </div>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      {/* Connect/Disconnect actions */}
      {showActions && state === 'disconnected' && (
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2"
          onClick={connect}
        >
          {t('connection.action.connect', 'Connect')}
        </Button>
      )}
      {showActions && state === 'connected' && (
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2 text-muted-foreground"
          onClick={disconnect}
        >
          {t('connection.action.disconnect', 'Disconnect')}
        </Button>
      )}
    </div>
  );
}

export default ConnectionStatus;
