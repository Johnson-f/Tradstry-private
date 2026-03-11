'use client';

import React, { createContext, useContext, useMemo, useState } from 'react';
import { useWebSocket } from '@/lib/hooks/use-websocket';
import type { WebSocketConnectionState, WebSocketEventType, WebSocketHandler } from '@/lib/websocket/types';

interface WebSocketContextValue {
  state: WebSocketConnectionState;
  error: Error | null;
  subscribe: (event: WebSocketEventType, handler: WebSocketHandler) => () => void;
  send: (message: unknown) => void;
}

const WebSocketContext = createContext<WebSocketContextValue | undefined>(undefined);
const WebSocketControlContext = createContext<{ enable: () => void; disable: () => void; } | undefined>(undefined);

interface WebSocketProviderProps {
  children: React.ReactNode;
}

export function WebSocketProvider({ children }: WebSocketProviderProps) {
  const [shouldConnect, setShouldConnect] = useState(false);
  
  console.log('WebSocketProvider render - shouldConnect:', shouldConnect);
  
  const { state, error, subscribe, send } = useWebSocket(shouldConnect);

  const controlValue = useMemo(() => ({
    enable: () => {
      console.log('🎮 WebSocket enable() called');
      setShouldConnect(true);
    },
    disable: () => {
      console.log('🎮 WebSocket disable() called');
      setShouldConnect(false);
    },
  }), []);

  const value = useMemo<WebSocketContextValue>(() => ({ state, error, subscribe, send }), [state, error, subscribe, send]);

  return (
    <WebSocketContext.Provider value={value}>
      <WebSocketControlContext.Provider value={controlValue}>
        {children}
      </WebSocketControlContext.Provider>
    </WebSocketContext.Provider>
  );
}

export function useWs() {
  const ctx = useContext(WebSocketContext);
  if (!ctx) throw new Error('useWs must be used within WebSocketProvider');
  return ctx;
}

export function useWebSocketControl() {
  const ctx = useContext(WebSocketControlContext);
  if (!ctx) throw new Error('useWebSocketControl must be used within WebSocketProvider');
  return ctx;
}