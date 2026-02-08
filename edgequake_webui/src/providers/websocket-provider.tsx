/**
 * @module WebSocketProvider
 * @description WebSocket connection context for real-time progress tracking.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 *
 * @implements FEAT0724 - Real-time ingestion progress
 * @implements FEAT0865 - WebSocket connection management
 *
 * @enforces BR0865 - Auto-reconnect on disconnect
 * @enforces BR0866 - Clean disconnect on unmount
 */
'use client';

import type { ProgressWebSocket } from '@/lib/websocket';
import { disconnectWebSocket, getWebSocketClient } from '@/lib/websocket';
import { useCostStore } from '@/stores/use-cost-store';
import { useIngestionStore } from '@/stores/use-ingestion-store';
import type { CostUpdateEvent } from '@/types/cost';
import type { IngestionFailedEvent, WebSocketProgressMessage } from '@/types/ingestion';
import { createContext, useCallback, useContext, useEffect, useRef, type ReactNode } from 'react';
import { toast } from 'sonner';

// ============================================================================
// Context Types
// ============================================================================

interface WebSocketContextValue {
  /** Whether the WebSocket is connected */
  connected: boolean;
  /** Whether the WebSocket is reconnecting */
  reconnecting: boolean;
  /** Subscribe to progress updates for track IDs */
  subscribe: (trackIds: string[]) => void;
  /** Unsubscribe from progress updates */
  unsubscribe: (trackIds: string[]) => void;
  /** Cancel an ingestion job */
  cancel: (trackId: string) => void;
  /** Manually connect to WebSocket */
  connect: () => void;
  /** Manually disconnect from WebSocket */
  disconnect: () => void;
}

const WebSocketContext = createContext<WebSocketContextValue | null>(null);

// ============================================================================
// Provider Component
// ============================================================================

interface WebSocketProviderProps {
  children: ReactNode;
  /** Whether to auto-connect on mount (default: true) */
  autoConnect?: boolean;
  /** Whether WebSocket is enabled (default: true) */
  enabled?: boolean;
}

export function WebSocketProvider({
  children,
  autoConnect = true,
  enabled = true,
}: WebSocketProviderProps) {
  const clientRef = useRef<ProgressWebSocket | null>(null);
  
  // Get store actions
  const { updateFromMessage, setWsConnected, setWsReconnecting, setWsMaxReconnectsReached } = useIngestionStore();
  const { updateIngestionCost } = useCostStore();
  
  // Track connection state locally for context value
  const connectedRef = useRef(false);
  const reconnectingRef = useRef(false);

  // Handle incoming messages
  const handleMessage = useCallback(
    (message: WebSocketProgressMessage | CostUpdateEvent) => {
      // Log stage transitions for debugging phase gaps
      if (message.type === 'stage_started') {
        console.log('[WebSocket] Stage started:', {
          track_id: (message as any).track_id,
          stage: (message as any).stage,
          document_id: (message as any).document_id,
        });
      } else if (message.type === 'stage_completed') {
        console.log('[WebSocket] Stage completed:', {
          track_id: (message as any).track_id,
          stage: (message as any).stage,
          duration_ms: (message as any).duration_ms,
        });
      } else if (message.type === 'stage_progress') {
        // Only log high progress to avoid spam
        const progress = (message as any).progress || 0;
        if (progress >= 95) {
          console.log('[WebSocket] Stage near completion:', {
            track_id: (message as any).track_id,
            stage: (message as any).stage,
            progress: `${progress}%`,
          });
        }
      }
      
      // Log pipeline events for debugging
      if (message.type === 'ingestion_failed') {
        const failedEvent = message as IngestionFailedEvent;
        console.error('[WebSocket] Ingestion failed:', {
          track_id: failedEvent.track_id,
          document_id: failedEvent.document_id,
          stage: failedEvent.stage,
          error: failedEvent.error,
        });
        
        // Show error toast to user
        toast.error(
          `Document processing failed: ${failedEvent.error.message}`,
          {
            duration: 10000, // 10 seconds
            description: `Stage: ${failedEvent.stage}`,
            action: failedEvent.error.recoverable ? {
              label: 'Retry',
              onClick: () => {
                // Track ID is available for retry logic
                console.log('[WebSocket] Retry requested for:', failedEvent.track_id);
              },
            } : undefined,
          }
        );
      } else if (message.type === 'ingestion_completed') {
        console.log('[WebSocket] Ingestion completed:', message);
      }
      
      // Update ingestion store
      updateFromMessage(message);
      
      // Handle cost updates separately
      if (message.type === 'cost_update') {
        const costMessage = message as CostUpdateEvent;
        updateIngestionCost(costMessage.track_id, costMessage.cumulative_cost_usd);
      }
    },
    [updateFromMessage, updateIngestionCost]
  );

  // Initialize WebSocket client
  useEffect(() => {
    if (!enabled) return;

    const client = getWebSocketClient();
    clientRef.current = client;

    // Set up event listeners
    const unsubConnected = client.on('connected', () => {
      connectedRef.current = true;
      reconnectingRef.current = false;
      setWsConnected(true);
      // OODA-02: Show reconnection success toast if we were disconnected
      if (useIngestionStore.getState().wsMaxReconnectsReached) {
        setWsMaxReconnectsReached(false);
        toast.success('Connection restored', {
          description: 'Real-time updates are back online.',
          duration: 3000,
        });
      }
    });

    const unsubDisconnected = client.on('disconnected', () => {
      connectedRef.current = false;
      setWsConnected(false);
      // OODA-02: Notify user of disconnection
      toast.warning('Connection lost', {
        description: 'Attempting to reconnect...',
        duration: 5000,
      });
    });

    const unsubReconnecting = client.on('reconnecting', () => {
      reconnectingRef.current = true;
      setWsReconnecting(true);
    });

    const unsubMaxReconnects = client.on('max_reconnects_reached', () => {
      reconnectingRef.current = false;
      setWsReconnecting(false);
      setWsMaxReconnectsReached(true);
      console.warn('[WebSocketProvider] Max reconnection attempts reached');
      // OODA-02: Show persistent error toast with retry option
      toast.error('Unable to reconnect', {
        description: 'Real-time updates unavailable. Click to retry.',
        duration: Infinity,
        action: {
          label: 'Retry',
          onClick: () => {
            setWsMaxReconnectsReached(false);
            clientRef.current?.connect();
          },
        },
      });
    });

    const unsubProgress = client.on('progress', (message) => {
      handleMessage(message as WebSocketProgressMessage);
    });

    const unsubPdfProgress = client.on('pdf_progress', (message) => {
      handleMessage(message as WebSocketProgressMessage);
    });

    const unsubStatusSnapshot = client.on('status_snapshot', (message) => {
      handleMessage(message as WebSocketProgressMessage);
    });

    // Auto-connect if enabled
    if (autoConnect) {
      client.connect();
    }

    // Cleanup
    return () => {
      unsubConnected();
      unsubDisconnected();
      unsubReconnecting();
      unsubMaxReconnects();
      unsubProgress();
      unsubPdfProgress();
      unsubStatusSnapshot();
    };
  }, [enabled, autoConnect, handleMessage, setWsConnected, setWsReconnecting, setWsMaxReconnectsReached]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnectWebSocket();
    };
  }, []);

  // Context value
  const subscribe = useCallback((trackIds: string[]) => {
    clientRef.current?.subscribe(trackIds);
  }, []);

  const unsubscribe = useCallback((trackIds: string[]) => {
    clientRef.current?.unsubscribe(trackIds);
  }, []);

  const cancel = useCallback((trackId: string) => {
    clientRef.current?.cancel(trackId);
  }, []);

  const connect = useCallback(() => {
    clientRef.current?.connect();
  }, []);

  const disconnect = useCallback(() => {
    clientRef.current?.disconnect();
  }, []);

  // Use store state for context value (not refs during render)
  const storeConnected = useIngestionStore((s) => s.wsConnected);
  const storeReconnecting = useIngestionStore((s) => s.wsReconnecting);

  const value: WebSocketContextValue = {
    connected: storeConnected,
    reconnecting: storeReconnecting,
    subscribe,
    unsubscribe,
    cancel,
    connect,
    disconnect,
  };

  return (
    <WebSocketContext.Provider value={value}>
      {children}
    </WebSocketContext.Provider>
  );
}

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to access WebSocket context.
 */
export function useWebSocketContext(): WebSocketContextValue {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocketContext must be used within a WebSocketProvider');
  }
  return context;
}

/**
 * Hook to get WebSocket connection status.
 * Can be used outside of WebSocketProvider (returns defaults).
 */
export function useWebSocketStatus(): { connected: boolean; reconnecting: boolean } {
  const { wsConnected, wsReconnecting } = useIngestionStore();
  return { connected: wsConnected, reconnecting: wsReconnecting };
}
