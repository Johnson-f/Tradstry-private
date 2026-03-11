'use client';

import { useEffect, useRef, useCallback, useState } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import * as Y from 'yjs';
import { WebsocketProvider } from 'y-websocket';
import type { CollaboratorPresence, CursorPosition } from '@/lib/types/collaboration';
import { apiConfig } from '@/lib/config/api';

// Connection status types
export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error';

interface CollaborationPluginProps {
  noteId: string;
  userId: string;
  userName: string;
  userColor: string;
  token: string;
  onAwarenessChange?: (states: Map<number, CollaboratorPresence>) => void;
  onSyncStatusChange?: (synced: boolean) => void;
  onConnectionStatusChange?: (status: ConnectionStatus) => void;
  onError?: (error: Error) => void;
  enabled?: boolean;
}

// Awareness state for cursor positions
interface AwarenessState {
  user: {
    id: string;
    name: string;
    color: string;
  };
  cursor: CursorPosition | null;
}

// Reconnection configuration
const RECONNECT_CONFIG = {
  initialDelay: 1000,      // 1 second
  maxDelay: 30000,         // 30 seconds max
  multiplier: 2,           // Double the delay each attempt
  maxAttempts: 10,         // Maximum reconnection attempts
  jitterFactor: 0.3,       // Add randomness to prevent thundering herd
};

// Calculate delay with exponential backoff and jitter
function calculateBackoffDelay(attempt: number): number {
  const baseDelay = Math.min(
    RECONNECT_CONFIG.initialDelay * Math.pow(RECONNECT_CONFIG.multiplier, attempt),
    RECONNECT_CONFIG.maxDelay
  );
  // Add jitter: random value between -jitterFactor and +jitterFactor of baseDelay
  const jitter = baseDelay * RECONNECT_CONFIG.jitterFactor * (Math.random() * 2 - 1);
  return Math.max(RECONNECT_CONFIG.initialDelay, baseDelay + jitter);
}

export function CollaborationPlugin({
  noteId,
  userId,
  userName,
  userColor,
  token,
  onAwarenessChange,
  onSyncStatusChange,
  onConnectionStatusChange,
  onError,
  enabled = true,
}: CollaborationPluginProps) {
  const [editor] = useLexicalComposerContext();
  const ydocRef = useRef<Y.Doc | null>(null);
  const providerRef = useRef<WebsocketProvider | null>(null);
  const isLocalChangeRef = useRef(false);
  const reconnectAttemptRef = useRef(0);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const connectionStatusRef = useRef<ConnectionStatus>('disconnected');
  const isManualDisconnectRef = useRef(false);
  const pendingUpdatesRef = useRef<Uint8Array[]>([]);

  // Update connection status and notify
  const setConnectionStatus = useCallback((status: ConnectionStatus) => {
    connectionStatusRef.current = status;
    onConnectionStatusChange?.(status);
  }, [onConnectionStatusChange]);

  // Handle connection error
  const handleError = useCallback((error: Error) => {
    console.error('[CollaborationPlugin] Error:', error.message);
    onError?.(error);
  }, [onError]);

  // Clear reconnection timeout
  const clearReconnectTimeout = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
  }, []);

  // Attempt reconnection with exponential backoff
  const attemptReconnect = useCallback(() => {
    if (isManualDisconnectRef.current) {
      return;
    }

    if (reconnectAttemptRef.current >= RECONNECT_CONFIG.maxAttempts) {
      setConnectionStatus('error');
      handleError(new Error(`Failed to reconnect after ${RECONNECT_CONFIG.maxAttempts} attempts`));
      return;
    }

    const delay = calculateBackoffDelay(reconnectAttemptRef.current);
    console.log(`[CollaborationPlugin] Reconnecting in ${Math.round(delay)}ms (attempt ${reconnectAttemptRef.current + 1}/${RECONNECT_CONFIG.maxAttempts})`);
    
    setConnectionStatus('reconnecting');

    reconnectTimeoutRef.current = setTimeout(() => {
      reconnectAttemptRef.current++;
      
      const provider = providerRef.current;
      if (provider && !provider.wsconnected) {
        provider.connect();
      }
    }, delay);
  }, [setConnectionStatus, handleError]);

  // Sync pending local changes after reconnection
  const syncPendingUpdates = useCallback(() => {
    const ydoc = ydocRef.current;
    if (!ydoc || pendingUpdatesRef.current.length === 0) return;

    console.log(`[CollaborationPlugin] Syncing ${pendingUpdatesRef.current.length} pending updates`);
    
    // The y-websocket provider handles syncing automatically on reconnect
    // by exchanging state vectors. We just need to ensure our local state
    // is properly merged. Clear pending updates as they're now synced.
    pendingUpdatesRef.current = [];
  }, []);

  // Initialize Yjs document and WebSocket provider
  useEffect(() => {
    if (!enabled || !noteId) return;

    isManualDisconnectRef.current = false;
    reconnectAttemptRef.current = 0;
    setConnectionStatus('connecting');

    const ydoc = new Y.Doc();
    ydocRef.current = ydoc;

    // Create shared type for editor content
    const yContent = ydoc.getMap('content');

    // Connect to WebSocket server using apiConfig
    const wsUrl = apiConfig.ws.collab(noteId, token);
    const provider = new WebsocketProvider(wsUrl, `note-${noteId}`, ydoc, {
      connect: true,
      // Disable y-websocket's built-in reconnect - we handle it ourselves
      resyncInterval: -1,
    });
    providerRef.current = provider;

    // Set up awareness for cursor positions
    const awareness = provider.awareness;
    awareness.setLocalState({
      user: { id: userId, name: userName, color: userColor },
      cursor: null,
    } as AwarenessState);

    // Handle awareness changes (other users' cursors)
    const handleAwarenessChange = () => {
      const states = new Map<number, CollaboratorPresence>();
      awareness.getStates().forEach((state, clientId) => {
        if (state.user && state.user.id !== userId) {
          states.set(clientId, {
            user_id: state.user.id,
            user_email: '',
            user_name: state.user.name,
            role: 'editor',
            cursor: state.cursor,
            is_active: true,
            joined_at: new Date().toISOString(),
            last_activity: new Date().toISOString(),
          });
        }
      });
      onAwarenessChange?.(states);
    };

    awareness.on('change', handleAwarenessChange);

    // Handle sync status
    provider.on('sync', (synced: boolean) => {
      onSyncStatusChange?.(synced);
      if (synced) {
        // Successfully synced - sync any pending updates
        syncPendingUpdates();
      }
    });

    // Handle WebSocket connection status
    provider.on('status', (event: { status: string }) => {
      console.log('[CollaborationPlugin] WebSocket status:', event.status);
      
      if (event.status === 'connected') {
        setConnectionStatus('connected');
        reconnectAttemptRef.current = 0; // Reset reconnect attempts on successful connection
        clearReconnectTimeout();
      } else if (event.status === 'disconnected') {
        if (!isManualDisconnectRef.current) {
          setConnectionStatus('disconnected');
          attemptReconnect();
        }
      }
    });

    // Handle connection close
    provider.on('connection-close', (event: CloseEvent | null) => {
      console.log('[CollaborationPlugin] Connection closed:', event?.code, event?.reason);
      
      if (!isManualDisconnectRef.current) {
        // Determine if this is a recoverable error
        if (event?.code === 1000) {
          // Normal closure - don't reconnect
          setConnectionStatus('disconnected');
        } else if (event?.code === 1008 || event?.code === 4001) {
          // Policy violation or authentication error - don't reconnect
          setConnectionStatus('error');
          handleError(new Error('Authentication failed. Please refresh the page.'));
        } else if (event?.code === 4003) {
          // Forbidden - no access
          setConnectionStatus('error');
          handleError(new Error('You do not have access to this note.'));
        } else {
          // Other errors - attempt reconnect
          attemptReconnect();
        }
      }
    });

    // Handle connection error
    provider.on('connection-error', (event: Event) => {
      console.error('[CollaborationPlugin] Connection error:', event);
      
      if (!isManualDisconnectRef.current) {
        attemptReconnect();
      }
    });

    // Handle remote changes from Yjs
    const handleYjsUpdate = () => {
      if (isLocalChangeRef.current) {
        isLocalChangeRef.current = false;
        return;
      }

      const content = yContent.get('editorState');
      if (content && typeof content === 'string') {
        try {
          editor.update(() => {
            const editorState = editor.parseEditorState(content);
            editor.setEditorState(editorState);
          }, { discrete: true, tag: 'collaboration' });
        } catch (e) {
          console.error('[CollaborationPlugin] Failed to apply remote changes:', e);
          handleError(new Error('Failed to apply remote changes'));
        }
      }
    };

    yContent.observe(handleYjsUpdate);

    // Cleanup
    return () => {
      isManualDisconnectRef.current = true;
      clearReconnectTimeout();
      awareness.off('change', handleAwarenessChange);
      yContent.unobserve(handleYjsUpdate);
      provider.disconnect();
      ydoc.destroy();
      ydocRef.current = null;
      providerRef.current = null;
      setConnectionStatus('disconnected');
    };
  }, [noteId, userId, userName, userColor, token, enabled, editor, onAwarenessChange, onSyncStatusChange, setConnectionStatus, handleError, attemptReconnect, clearReconnectTimeout, syncPendingUpdates]);

  // Broadcast local changes to Yjs
  useEffect(() => {
    if (!enabled || !ydocRef.current) return;

    const removeListener = editor.registerUpdateListener(({ editorState, tags }) => {
      // Skip if this update came from collaboration
      if (tags.has('collaboration')) return;

      const ydoc = ydocRef.current;
      if (!ydoc) return;

      isLocalChangeRef.current = true;
      const yContent = ydoc.getMap('content');
      const serialized = JSON.stringify(editorState.toJSON());
      yContent.set('editorState', serialized);

      // If disconnected, store update for later sync
      if (connectionStatusRef.current !== 'connected') {
        const update = Y.encodeStateAsUpdate(ydoc);
        pendingUpdatesRef.current.push(update);
      }
    });

    return removeListener;
  }, [editor, enabled]);

  // Update cursor position in awareness
  const updateCursor = useCallback((cursor: CursorPosition | null) => {
    const provider = providerRef.current;
    if (!provider) return;

    const currentState = provider.awareness.getLocalState() as AwarenessState;
    provider.awareness.setLocalState({
      ...currentState,
      cursor,
    });
  }, []);

  // Track selection changes for cursor position
  useEffect(() => {
    if (!enabled) return;

    const removeListener = editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const selection = editorState._selection;
        if (selection) {
          // Simplified cursor position - in production you'd calculate line/column
          updateCursor({
            user_id: userId,
            user_name: userName,
            user_color: userColor,
            line: 0,
            column: 0,
          });
        }
      });
    });

    return removeListener;
  }, [editor, enabled, userId, userName, userColor, updateCursor]);

  return null;
}

export default CollaborationPlugin;
