'use client';

/**
 * WebSocket Provider
 * 
 * Provides WebSocket connection context for real-time progress tracking.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 */

import type { ProgressWebSocket } from '@/lib/websocket';
import { disconnectWebSocket, getWebSocketClient } from '@/lib/websocket';
import { useCostStore } from '@/stores/use-cost-store';
import { useIngestionStore } from '@/stores/use-ingestion-store';
import type { CostUpdateEvent } from '@/types/cost';
import type { WebSocketProgressMessage } from '@/types/ingestion';
import { createContext, useCallback, useContext, useEffect, useRef, type ReactNode } from 'react';

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
  const { updateFromMessage, setWsConnected, setWsReconnecting } = useIngestionStore();
  const { updateIngestionCost } = useCostStore();
  
  // Track connection state locally for context value
  const connectedRef = useRef(false);
  const reconnectingRef = useRef(false);

  // Handle incoming messages
  const handleMessage = useCallback(
    (message: WebSocketProgressMessage | CostUpdateEvent) => {
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
    });

    const unsubDisconnected = client.on('disconnected', () => {
      connectedRef.current = false;
      setWsConnected(false);
    });

    const unsubReconnecting = client.on('reconnecting', () => {
      reconnectingRef.current = true;
      setWsReconnecting(true);
    });

    const unsubMaxReconnects = client.on('max_reconnects_reached', () => {
      reconnectingRef.current = false;
      setWsReconnecting(false);
      console.warn('[WebSocketProvider] Max reconnection attempts reached');
    });

    const unsubProgress = client.on('progress', (message) => {
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
    };
  }, [enabled, autoConnect, handleMessage, setWsConnected, setWsReconnecting]);

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
