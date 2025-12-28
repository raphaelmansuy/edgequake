/**
 * WebSocket Hook
 *
 * Hook for accessing WebSocket connection status and actions.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
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
