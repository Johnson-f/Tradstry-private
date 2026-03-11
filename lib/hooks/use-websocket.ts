'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { createClient } from '@/lib/supabase/client';
import { apiConfig } from '@/lib/config/api';
import type {
  WebSocketConnectionState,
  WebSocketEnvelope,
  WebSocketEventType,
  WebSocketHandler,
} from '@/lib/websocket/types';

interface UseWebSocketReturn {
  state: WebSocketConnectionState;
  error: Error | null;
  send: (message: unknown) => void;
  subscribe: (event: WebSocketEventType, handler: WebSocketHandler) => () => void;
}

export function useWebSocket(shouldConnect: boolean = true): UseWebSocketReturn {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const handlersRef = useRef<Map<WebSocketEventType, Set<WebSocketHandler>>>(new Map());
  const [state, setState] = useState<WebSocketConnectionState>(shouldConnect ? 'connecting' : 'disconnected');
  const [error, setError] = useState<Error | null>(null);

  console.log('useWebSocket render - shouldConnect:', shouldConnect, 'state:', state);

  const connect = useCallback(async () => {
    // Don't connect if we shouldn't be connecting
    if (!shouldConnect) {
      console.log('❌ Not connecting - shouldConnect is false');
      setState('disconnected');
      return;
    }

    console.log('🔌 Attempting to connect WebSocket...');

    try {
      setState('connecting');
      setError(null);

      const { data: { session } } = await createClient().auth.getSession();
      const token = session?.access_token;
      if (!token) throw new Error('Missing auth token');

      const wsUrl = `${apiConfig.baseURL.replace('http', 'ws')}${apiConfig.apiPrefix}/ws?token=${encodeURIComponent(token)}`;
      
      console.log('🔌 Connecting to:', wsUrl);

      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        console.log('✅ WebSocket connected');
        setState('connected');
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = (event) => {
        try {
          console.log('📨 Raw WebSocket message received:', event.data);
          const envelope = JSON.parse(event.data) as WebSocketEnvelope;
          const { event: ev, data } = envelope;
          console.log('📦 Parsed envelope - event:', ev, 'data:', data);
          // Normalize server event names like "stock_created" -> "stock:created"
          // Replace all underscores with colons for snake_case to colon format
          const normalized: WebSocketEventType = (typeof ev === 'string'
            ? (ev as string).replace(/_/g, ':')
            : ev) as WebSocketEventType;
          console.log('🔄 Normalized event type:', normalized);
          const set = handlersRef.current.get(normalized);
          if (set) {
            console.log(`✅ Found ${set.size} handler(s) for event: ${normalized}`);
            set.forEach((handler) => handler(data));
          } else {
            console.warn(`⚠️ No handlers registered for event: ${normalized}`);
            console.log('Available handlers:', Array.from(handlersRef.current.keys()));
          }
        } catch (e) {
          console.error('❌ Error parsing WebSocket message:', e);
          // Non-enveloped message; ignore
        }
      };

      ws.onerror = (err) => {
        console.error('❌ WebSocket error:', err);
        setState('error');
        setError(new Error('WebSocket error'));
      };

      ws.onclose = () => {
        console.log('🔌 WebSocket closed');
        setState('disconnected');
        
        // Only reconnect if shouldConnect is still true
        if (shouldConnect) {
          // Reconnect with exponential backoff up to ~10s
          const attempt = reconnectAttemptsRef.current++;
          const delay = Math.min(1000 * Math.pow(2, attempt), 10000);
          console.log(`🔄 Reconnecting in ${delay}ms (attempt ${attempt + 1})`);
          setTimeout(() => connect(), delay);
        }
      };
    } catch (e) {
      console.error('❌ WebSocket connection error:', e);
      setError(e as Error);
      setState('error');
    }
  }, [shouldConnect]);

  useEffect(() => {
    console.log('useWebSocket effect - shouldConnect changed to:', shouldConnect);
    
    if (shouldConnect) {
      connect();
    } else {
      console.log('Closing existing connection (shouldConnect is false)');
      wsRef.current?.close();
      wsRef.current = null;
      setState('disconnected');
    }
    
    return () => {
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [connect, shouldConnect]);

  const send = useCallback((message: unknown) => {
    if (wsRef.current && state === 'connected') {
      wsRef.current.send(JSON.stringify(message));
    }
  }, [state]);

  const subscribe = useCallback((event: WebSocketEventType, handler: WebSocketHandler) => {
    if (!handlersRef.current.has(event)) {
      handlersRef.current.set(event, new Set());
    }
    const set = handlersRef.current.get(event)!;
    set.add(handler);
    return () => {
      set.delete(handler);
      if (set.size === 0) handlersRef.current.delete(event);
    };
  }, []);

  return { state, error, send, subscribe };
}