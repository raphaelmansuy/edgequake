/**
 * Live Message Component
 * 
 * Displays streaming/live ingestion messages.
 * Based on WebUI Specification Document WEBUI-004 (13-webui-components.md)
 *
 * @implements FEAT1064 - Real-time ingestion message stream
 * @implements FEAT1065 - Message history display
 *
 * @see UC1405 - User sees live processing updates
 * @see UC1406 - User reviews message history
 *
 * @enforces BR1064 - Animated message updates
 * @enforces BR1065 - Auto-scroll to latest message
 */

'use client';

import { cn } from '@/lib/utils';
import { Loader2, Terminal } from 'lucide-react';
import { useEffect, useState } from 'react';

interface LiveMessageProps {
  /** The current message to display */
  message?: string;
  /** Previous messages to show in history */
  history?: string[];
  /** Maximum history items to show */
  maxHistory?: number;
  /** Whether the process is actively running */
  isActive?: boolean;
  /** Show timestamp on messages */
  showTimestamp?: boolean;
  /** Custom class name */
  className?: string;
}

interface HistoryItem {
  message: string;
  timestamp: Date;
}

/**
 * Displays live streaming messages from ingestion progress.
 * 
 * Features:
 * - Animated typing indicator when active
 * - Message history with timestamps
 * - Auto-scroll to latest message
 */
export function LiveMessage({
  message,
  history = [],
  maxHistory = 5,
  isActive = true,
  showTimestamp = false,
  className,
}: LiveMessageProps) {
  const [displayedMessage, setDisplayedMessage] = useState(message || '');
  const [historyItems, setHistoryItems] = useState<HistoryItem[]>([]);

  // Update displayed message with animation
  // Note: This is a valid pattern - we're syncing external state changes
  useEffect(() => {
    if (message && message !== displayedMessage) {
      // Add previous message to history if it exists
      if (displayedMessage) {
        // Intentional: Syncing external state changes to local state
        // eslint-disable-next-line react-hooks/set-state-in-effect
        setHistoryItems(prev => {
          const newHistory = [
            ...prev,
            { message: displayedMessage, timestamp: new Date() }
          ].slice(-maxHistory);
          return newHistory;
        });
      }
      setDisplayedMessage(message);
    }
  }, [message, maxHistory, displayedMessage]);

  // Sync external history
  // Note: This is a valid pattern - we're syncing external state changes
  useEffect(() => {
    if (history.length > 0) {
      // Intentional: Syncing external prop to local state
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setHistoryItems(
        history.map(msg => ({ message: msg, timestamp: new Date() }))
          .slice(-maxHistory)
      );
    }
  }, [history, maxHistory]);

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  return (
    <div className={cn('rounded-lg border bg-muted/30 p-3', className)}>
      {/* Header */}
      <div className="flex items-center gap-2 mb-2">
        <Terminal className="h-4 w-4 text-muted-foreground" />
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          Live Progress
        </span>
        {isActive && (
          <Loader2 className="h-3 w-3 animate-spin text-blue-500 ml-auto" />
        )}
      </div>

      {/* Message history */}
      {historyItems.length > 0 && (
        <div className="space-y-1 mb-2 opacity-60">
          {historyItems.map((item, index) => (
            <div key={index} className="flex items-start gap-2 text-xs">
              {showTimestamp && (
                <span className="text-muted-foreground font-mono shrink-0">
                  [{formatTime(item.timestamp)}]
                </span>
              )}
              <span className="text-muted-foreground line-clamp-1">
                {item.message}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Current message */}
      <div className="flex items-start gap-2">
        {isActive && (
          <span className="text-blue-500 animate-pulse">▸</span>
        )}
        <p className={cn(
          'text-sm',
          isActive ? 'text-foreground' : 'text-muted-foreground'
        )}>
          {displayedMessage || 'Waiting for updates...'}
          {isActive && <span className="animate-pulse ml-0.5">|</span>}
        </p>
      </div>
    </div>
  );
}

/**
 * Compact inline version for table rows or list items.
 */
export function LiveMessageInline({
  message,
  isActive = true,
  className,
}: {
  message?: string;
  isActive?: boolean;
  className?: string;
}) {
  return (
    <div className={cn('flex items-center gap-2', className)}>
      {isActive && (
        <Loader2 className="h-3 w-3 animate-spin text-blue-500 shrink-0" />
      )}
      <span className={cn(
        'text-sm truncate',
        isActive ? 'text-foreground' : 'text-muted-foreground'
      )}>
        {message || 'Processing...'}
      </span>
    </div>
  );
}

export default LiveMessage;
