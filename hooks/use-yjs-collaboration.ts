'use client';

import { useState, useCallback, useRef, useEffect } from 'react';
import type { CollaboratorPresence } from '@/lib/types/collaboration';
import type { ConnectionStatus } from '@/components/notebook/rich_editor/plugins';

interface UseYjsCollaborationOptions {
  noteId: string;
  userId: string;
  userName: string;
  userColor: string;
  token: string;
  enabled?: boolean;
}

interface UseYjsCollaborationReturn {
  // Connection state
  connectionStatus: ConnectionStatus;
  isSynced: boolean;
  error: string | null;
  
  // Participants from awareness
  participants: Map<number, CollaboratorPresence>;
  
  // Callbacks for CollaborationPlugin
  onConnectionStatusChange: (status: ConnectionStatus) => void;
  onSyncStatusChange: (synced: boolean) => void;
  onAwarenessChange: (states: Map<number, CollaboratorPresence>) => void;
  onError: (error: Error) => void;
  
  // Actions
  clearError: () => void;
  retry: () => void;
}

export function useYjsCollaboration({
  noteId,
  userId,
  userName,
  userColor,
  token,
  enabled = true,
}: UseYjsCollaborationOptions): UseYjsCollaborationReturn {
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected');
  const [isSynced, setIsSynced] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [participants, setParticipants] = useState<Map<number, CollaboratorPresence>>(new Map());
  
  // Track retry trigger
  const retryCountRef = useRef(0);
  const [retryTrigger, setRetryTrigger] = useState(0);

  // Callbacks for CollaborationPlugin
  const onConnectionStatusChange = useCallback((status: ConnectionStatus) => {
    setConnectionStatus(status);
    
    // Clear error when connected
    if (status === 'connected') {
      setError(null);
    }
  }, []);

  const onSyncStatusChange = useCallback((synced: boolean) => {
    setIsSynced(synced);
  }, []);

  const onAwarenessChange = useCallback((states: Map<number, CollaboratorPresence>) => {
    setParticipants(new Map(states));
  }, []);

  const onError = useCallback((err: Error) => {
    setError(err.message);
    console.error('[useYjsCollaboration] Error:', err.message);
  }, []);

  // Clear error
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Retry connection - this will trigger a re-mount of CollaborationPlugin
  const retry = useCallback(() => {
    retryCountRef.current++;
    setRetryTrigger(retryCountRef.current);
    setError(null);
    setConnectionStatus('connecting');
  }, []);

  // Reset state when noteId changes
  useEffect(() => {
    setConnectionStatus('disconnected');
    setIsSynced(false);
    setError(null);
    setParticipants(new Map());
  }, [noteId]);

  return {
    connectionStatus,
    isSynced,
    error,
    participants,
    onConnectionStatusChange,
    onSyncStatusChange,
    onAwarenessChange,
    onError,
    clearError,
    retry,
  };
}

export default useYjsCollaboration;
