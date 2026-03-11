// AI Chat Hook with TanStack Query
// React hook for managing AI chat state and interactions using TanStack Query
"use client";
import { useState, useCallback, useMemo, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { aiChatService } from '@/lib/services/ai-chat-service';
import { createClient } from '@/lib/supabase/client';
import { useWs } from '@/lib/websocket/provider';
import type {
  ChatMessage,
  ChatError,
  UseAIChatReturn,
  ChatRequest,
  StreamingChatRequest,
} from '@/lib/types/ai-chat';
import { MessageRole } from '@/lib/types/ai-chat';

// Query keys
const QUERY_KEYS = {
  sessions: ['ai-chat', 'sessions'] as const,
  session: (id: string) => ['ai-chat', 'session', id] as const,
} as const;

export function useAIChat(): UseAIChatReturn {
  const queryClient = useQueryClient();
  const { subscribe } = useWs();
  
  // Local state for current session and streaming
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingMessage, setStreamingMessage] = useState<string>('');
  const [localError, setLocalError] = useState<ChatError | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null);

  // Check authentication status
  const checkAuth = useCallback(async () => {
    try {
      const supabase = createClient();
      const { data: { user }, error } = await supabase.auth.getUser();
      setIsAuthenticated(!!user && !error);
    } catch (error) {
      console.error('Error checking authentication:', error);
      setIsAuthenticated(false);
    }
  }, []);

  // Check auth on mount
  useMemo(() => {
    checkAuth();
  }, [checkAuth]);

  // Subscribe to WebSocket chat events for real-time updates
  useEffect(() => {
    const unsubscribeMessageCreated = subscribe('chat:message:created', (data: unknown) => {
      console.log('WebSocket: chat:message:created', data);
      
      // Invalidate queries to refetch with new message
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
      if (currentSessionId) {
        queryClient.invalidateQueries({ 
          queryKey: QUERY_KEYS.session(currentSessionId) 
        });
      }
    });

    const unsubscribeSessionCreated = subscribe('chat:session:created', (data: unknown) => {
      console.log('WebSocket: chat:session:created', data);
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
    });

    const unsubscribeSessionUpdated = subscribe('chat:session:updated', (data: unknown) => {
      console.log('WebSocket: chat:session:updated', data);
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
    });

    return () => {
      unsubscribeMessageCreated();
      unsubscribeSessionCreated();
      unsubscribeSessionUpdated();
    };
  }, [subscribe, queryClient, currentSessionId]);

  // Queries
  const {
    data: sessionsData,
    isLoading: isLoadingSessions,
    error: sessionsError,
    refetch: refetchSessions,
  } = useQuery({
    queryKey: QUERY_KEYS.sessions,
    queryFn: () => aiChatService.getSessions({ limit: 50 }),
    staleTime: 30 * 1000, // 30 seconds
    enabled: isAuthenticated === true, // Only run when authenticated
  });

  const sessions = sessionsData?.sessions || [];
  const totalSessionsCount = sessionsData?.total_count || 0;

  const {
    data: currentSessionData,
    isLoading: isLoadingSession,
    error: sessionError,
  } = useQuery({
    queryKey: QUERY_KEYS.session(currentSessionId!),
    queryFn: () => aiChatService.getSession(currentSessionId!),
    enabled: !!currentSessionId && isAuthenticated === true, // Only run when session exists and authenticated
    select: (data) => ({
      session: data.session,
      messages: data.messages,
    }),
  });

  // Mutations
  const sendMessageMutation = useMutation({
    mutationFn: (request: ChatRequest) => aiChatService.sendMessage(request),
    onSuccess: (response) => {
      // Invalidate and refetch sessions to update last message time
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
      
      // If session changed, update current session
      if (response.session.id !== currentSessionId) {
        setCurrentSessionId(response.session.id);
      }
      
      // Invalidate current session to refetch messages
      if (currentSessionId) {
        queryClient.invalidateQueries({ 
          queryKey: QUERY_KEYS.session(currentSessionId) 
        });
      }
    },
  });

  const createSessionMutation = useMutation({
    mutationFn: (request: { title?: string }) => aiChatService.createSession(request),
    onSuccess: (session) => {
      setCurrentSessionId(session.id);
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
    },
  });

  const updateSessionTitleMutation = useMutation({
    mutationFn: ({ sessionId, title }: { sessionId: string; title: string }) =>
      aiChatService.updateSessionTitle(sessionId, title),
    onSuccess: (_, { sessionId }) => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
      if (sessionId === currentSessionId) {
        queryClient.invalidateQueries({ 
          queryKey: QUERY_KEYS.session(sessionId) 
        });
      }
    },
  });

  const deleteSessionMutation = useMutation({
    mutationFn: (sessionId: string) => aiChatService.deleteSession(sessionId),
    onSuccess: (_, sessionId) => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
      
      // Clear current session if it's the one being deleted
      if (sessionId === currentSessionId) {
        setCurrentSessionId(null);
      }
    },
  });

  // Computed values
  const currentSession = currentSessionData?.session || null;
  const messages = currentSessionData?.messages || [];
  const isLoading = isLoadingSessions || isLoadingSession || 
    sendMessageMutation.isPending || createSessionMutation.isPending ||
    updateSessionTitleMutation.isPending || deleteSessionMutation.isPending;
  
  const error = useMemo(() => {
    // Return local error if set (for streaming errors)
    if (localError) return localError;
    
    // Otherwise return query/mutation errors
    if (sessionsError) return { message: sessionsError.message, code: 'SESSIONS_ERROR' };
    if (sessionError) return { message: sessionError.message, code: 'SESSION_ERROR' };
    if (sendMessageMutation.error) return { message: sendMessageMutation.error.message, code: 'SEND_MESSAGE_ERROR' };
    if (createSessionMutation.error) return { message: createSessionMutation.error.message, code: 'CREATE_SESSION_ERROR' };
    if (updateSessionTitleMutation.error) return { message: updateSessionTitleMutation.error.message, code: 'UPDATE_TITLE_ERROR' };
    if (deleteSessionMutation.error) return { message: deleteSessionMutation.error.message, code: 'DELETE_SESSION_ERROR' };
    return null;
  }, [localError, sessionsError, sessionError, sendMessageMutation.error, createSessionMutation.error, updateSessionTitleMutation.error, deleteSessionMutation.error]);

  // Actions
  const sendMessage = useCallback(async (
    message: string, 
    sessionId?: string,
    options?: { use_tools?: boolean; include_context?: boolean; max_context_vectors?: number }
  ) => {
    if (!message.trim()) return;

    const request: ChatRequest = {
      message: message.trim(),
      session_id: sessionId || currentSessionId || undefined,
      include_context: options?.include_context ?? true,
      max_context_vectors: options?.max_context_vectors ?? 10,
      use_tools: options?.use_tools,
    };

    await sendMessageMutation.mutateAsync(request);
  }, [currentSessionId, sendMessageMutation]);

  const sendStreamingMessage = useCallback(async (
    message: string, 
    sessionId?: string,
    options?: { use_tools?: boolean; include_context?: boolean; max_context_vectors?: number }
  ) => {
    console.log('sendStreamingMessage called with:', { message, sessionId, options });
    
    if (!message.trim()) return;

    setIsStreaming(true);
    setStreamingMessage('');
    setLocalError(null);

    try {
      const request: StreamingChatRequest = {
        message: message.trim(),
        session_id: sessionId || currentSessionId || undefined,
        include_context: options?.include_context ?? true,
        max_context_vectors: options?.max_context_vectors ?? 10,
        use_tools: options?.use_tools,
      };

      console.log('Sending streaming request:', request);
      let fullContent = '';

      await aiChatService.sendStreamingMessage(
        request,
        // onChunk
        (chunk: string) => {
          console.log('onChunk called with:', chunk);
          fullContent += chunk;
          console.log('Updated fullContent:', fullContent);
          setStreamingMessage(fullContent);
        },
        // onComplete
        async () => {
          console.log('Stream completed, invalidating queries...');
          setIsStreaming(false);
          setStreamingMessage('');
          
          // Wait for backend to save
          await new Promise(resolve => setTimeout(resolve, 2000));
          
          // Invalidate queries
          queryClient.invalidateQueries({ queryKey: QUERY_KEYS.sessions });
          if (sessionId || currentSessionId) {
            queryClient.invalidateQueries({ 
              queryKey: QUERY_KEYS.session(sessionId || currentSessionId!) 
            });
          }
        },
        // onError
        (err: Error) => {
          console.error('Stream error:', err);
          setIsStreaming(false);
          setStreamingMessage('');
          setLocalError({ message: err.message, code: 'STREAMING_ERROR' });
        }
      );
    } catch (err) {
      setIsStreaming(false);
      setStreamingMessage('');
      setLocalError({ message: (err as Error).message, code: 'STREAMING_ERROR' });
    }
  }, [currentSessionId, queryClient]);

  const createSession = useCallback(async (title?: string): Promise<string> => {
    const session = await createSessionMutation.mutateAsync({ title });
    return session.id;
  }, [createSessionMutation]);

  const updateSessionTitle = useCallback(async (sessionId: string, title: string) => {
    await updateSessionTitleMutation.mutateAsync({ sessionId, title });
  }, [updateSessionTitleMutation]);

  const deleteSession = useCallback(async (sessionId: string) => {
    await deleteSessionMutation.mutateAsync(sessionId);
  }, [deleteSessionMutation]);

  const loadSessions = useCallback(async () => {
    await refetchSessions();
  }, [refetchSessions]);

  const loadSession = useCallback(async (sessionId: string) => {
    setCurrentSessionId(sessionId);
  }, []);

  const clearError = useCallback(() => {
    // Clear local error
    setLocalError(null);
    
    // Clear all mutation errors
    sendMessageMutation.reset();
    createSessionMutation.reset();
    updateSessionTitleMutation.reset();
    deleteSessionMutation.reset();
  }, [sendMessageMutation, createSessionMutation, updateSessionTitleMutation, deleteSessionMutation]);

  // Combine messages with streaming message if active
  const allMessages = useMemo(() => {
    if (isStreaming && streamingMessage) {
      const streamingMsg: ChatMessage = {
        id: `streaming-${Date.now()}`,
        session_id: currentSessionId || 'temp',
        role: MessageRole.Assistant,
        content: streamingMessage,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };
      return [...messages, streamingMsg];
    }
    return messages;
  }, [messages, isStreaming, streamingMessage, currentSessionId]);

  return {
    // State
    sessions,
    totalSessionsCount,
    currentSession,
    messages: allMessages,
    isLoading,
    isStreaming,
    error,
    
    // Actions
    sendMessage,
    sendStreamingMessage,
    createSession,
    updateSessionTitle,
    deleteSession,
    loadSessions,
    loadSession,
    clearError,
  };
}
