/**
 * @module use-websocket
 * @description Hook for WebSocket connection management.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 *
 * @implements FEAT0603 - WebSocket real-time communication
 * @implements FEAT0605 - Connection state management
 * @implements FEAT0606 - Auto-reconnection with backoff
 *
 * @enforces BR0604 - Reconnect automatically on disconnect
 * @enforces BR0605 - State syncs across hooks
 *
 * @see {@link specs/WEBUI-005.md} for specification
 */

import { getWebSocketClient } from "@/lib/websocket";
import { useIngestionStore } from "@/stores/use-ingestion-store";
import { useCallback } from "react";

/**
 * Hook to get WebSocket connection status and actions.
 */
export function useWebSocket() {
  const { wsConnected, wsReconnecting } = useIngestionStore();

  const subscribe = useCallback((trackIds: string[]) => {
    const client = getWebSocketClient();
    client.subscribe(trackIds);
  }, []);

  const unsubscribe = useCallback((trackIds: string[]) => {
    const client = getWebSocketClient();
    client.unsubscribe(trackIds);
  }, []);

  const cancel = useCallback((trackId: string) => {
    const client = getWebSocketClient();
    client.cancel(trackId);
  }, []);

  const connect = useCallback(() => {
    const client = getWebSocketClient();
    client.connect();
  }, []);

  const disconnect = useCallback(() => {
    const client = getWebSocketClient();
    client.disconnect();
  }, []);

  return {
    connected: wsConnected,
    reconnecting: wsReconnecting,
    subscribe,
    unsubscribe,
    cancel,
    connect,
    disconnect,
  };
}

export default useWebSocket;
