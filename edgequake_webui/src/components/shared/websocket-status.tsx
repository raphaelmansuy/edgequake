/**
 * @module WebSocketStatus
 * @description Displays WebSocket connection status with visual indicator.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 * 
 * @implements FEAT0603 - WebSocket connection indicator
 * @implements FEAT0638 - Visual reconnection status
 * 
 * @enforces BR0604 - Status updates in real-time
 * @enforces BR0624 - Reconnection attempts visible to user
 * 
 * @see {@link specs/WEBUI-005.md} for specification
 */

'use client';

import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useWebSocket } from '@/hooks/use-websocket';
import { cn } from '@/lib/utils';
import { Loader2, Wifi, WifiOff } from 'lucide-react';

interface WebSocketStatusProps {
  /** Show label text alongside icon */
  showLabel?: boolean;
  /** Custom class name */
  className?: string;
  /** Size variant */
  size?: 'sm' | 'default' | 'lg';
}

/**
 * WebSocket connection status indicator component.
 * 
 * Shows a visual badge indicating the current WebSocket connection state:
 * - Connected: Green indicator with Wifi icon
 * - Reconnecting: Yellow indicator with loading spinner
 * - Disconnected: Red indicator with WifiOff icon
 */
export function WebSocketStatus({ 
  showLabel = false, 
  className,
  size = 'default'
}: WebSocketStatusProps) {
  const { connected, reconnecting } = useWebSocket();

  // Determine status display
  const status = reconnecting 
    ? 'reconnecting' 
    : connected 
      ? 'connected' 
      : 'disconnected';

  const statusConfig = {
    connected: {
      label: 'Live',
      variant: 'default' as const,
      icon: Wifi,
      color: 'bg-green-500',
      tooltip: 'Real-time updates active',
    },
    reconnecting: {
      label: 'Reconnecting',
      variant: 'secondary' as const,
      icon: Loader2,
      color: 'bg-yellow-500',
      tooltip: 'Attempting to reconnect...',
    },
    disconnected: {
      label: 'Offline',
      variant: 'destructive' as const,
      icon: WifiOff,
      color: 'bg-red-500',
      tooltip: 'Using polling fallback',
    },
  };

  const config = statusConfig[status];
  const Icon = config.icon;

  const iconSize = {
    sm: 'h-3 w-3',
    default: 'h-4 w-4',
    lg: 'h-5 w-5',
  };

  const badgePadding = {
    sm: 'px-1.5 py-0.5 text-xs',
    default: 'px-2 py-1',
    lg: 'px-3 py-1.5 text-sm',
  };

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Badge 
          variant={config.variant}
          className={cn(
            'flex items-center gap-1 cursor-help',
            badgePadding[size],
            className
          )}
        >
          {/* Status dot */}
          <span 
            className={cn(
              'rounded-full',
              size === 'sm' ? 'h-1.5 w-1.5' : size === 'lg' ? 'h-2.5 w-2.5' : 'h-2 w-2',
              config.color,
              status === 'reconnecting' && 'animate-pulse'
            )} 
          />
          
          {/* Icon */}
          <Icon 
            className={cn(
              iconSize[size],
              status === 'reconnecting' && 'animate-spin'
            )} 
          />
          
          {/* Optional label */}
          {showLabel && (
            <span className="font-medium">{config.label}</span>
          )}
        </Badge>
      </TooltipTrigger>
      <TooltipContent>
        <p>{config.tooltip}</p>
      </TooltipContent>
    </Tooltip>
  );
}

/**
 * Minimal status dot for compact layouts.
 */
export function WebSocketStatusDot({ className }: { className?: string }) {
  const { connected, reconnecting } = useWebSocket();

  const color = reconnecting 
    ? 'bg-yellow-500' 
    : connected 
      ? 'bg-green-500' 
      : 'bg-red-500';

  const tooltip = reconnecting 
    ? 'Reconnecting...' 
    : connected 
      ? 'Live updates active' 
      : 'Offline - using polling';

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span 
          className={cn(
            'inline-block h-2 w-2 rounded-full cursor-help',
            color,
            reconnecting && 'animate-pulse',
            className
          )} 
        />
      </TooltipTrigger>
      <TooltipContent>
        <p>{tooltip}</p>
      </TooltipContent>
    </Tooltip>
  );
}

export default WebSocketStatus;
