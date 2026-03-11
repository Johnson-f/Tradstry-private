'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { collaborationService } from '@/lib/services/collaboration-service';
import type {
  NoteCollaborator,
  NoteInvitation,
  NoteShareSettings,
  CollaboratorPresence,
  CursorPosition,
  NoteVisibility,
  CollaboratorRole,
  ContentUpdatePayload,
  CursorUpdatePayload,
} from '@/lib/types/collaboration';

interface UseCollaborationOptions {
  noteId: string;
  enabled?: boolean;
  onContentUpdate?: (payload: ContentUpdatePayload) => void;
  onCursorUpdate?: (payload: CursorUpdatePayload) => void;
  onParticipantJoin?: (participant: CollaboratorPresence) => void;
  onParticipantLeave?: (userId: string) => void;
}

interface UseCollaborationReturn {
  // State
  collaborators: NoteCollaborator[];
  participants: CollaboratorPresence[];
  shareSettings: NoteShareSettings | null;
  isLoading: boolean;
  error: string | null;
  isConnected: boolean;

  // Actions
  inviteCollaborator: (email: string, role: CollaboratorRole, message?: string, inviterName?: string, noteTitle?: string) => Promise<void>;
  removeCollaborator: (collaboratorId: string) => Promise<void>;
  updateRole: (collaboratorId: string, role: CollaboratorRole) => Promise<void>;
  updateVisibility: (visibility: NoteVisibility) => Promise<void>;
  updateCursor: (line: number, column: number, selectionStart?: number, selectionEnd?: number) => void;
  broadcastContent: (content: unknown, version: number) => void;
  joinRoom: () => Promise<void>;
  leaveRoom: () => Promise<void>;
  refresh: () => Promise<void>;
}

export function useCollaboration({
  noteId,
  enabled = true,
  onContentUpdate,
  onCursorUpdate,
  onParticipantJoin,
  onParticipantLeave,
}: UseCollaborationOptions): UseCollaborationReturn {
  const [collaborators, setCollaborators] = useState<NoteCollaborator[]>([]);
  const [participants, setParticipants] = useState<CollaboratorPresence[]>([]);
  const [shareSettings, setShareSettings] = useState<NoteShareSettings | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  const wsRef = useRef<WebSocket | null>(null);
  const cursorThrottleRef = useRef<NodeJS.Timeout | null>(null);
  const contentThrottleRef = useRef<NodeJS.Timeout | null>(null);

  // Fetch collaborators and settings
  const fetchData = useCallback(async () => {
    if (!noteId || !enabled) return;

    setIsLoading(true);
    setError(null);

    try {
      const [collabs, settings] = await Promise.all([
        collaborationService.getCollaborators(noteId),
        collaborationService.getShareSettings(noteId),
      ]);
      setCollaborators(collabs);
      setShareSettings(settings);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load collaboration data');
    } finally {
      setIsLoading(false);
    }
  }, [noteId, enabled]);

  // Join collaboration room
  const joinRoom = useCallback(async () => {
    if (!noteId || !enabled) return;

    try {
      const { participants: roomParticipants } = await collaborationService.joinRoom(noteId);
      setParticipants(roomParticipants);
      setIsConnected(true);
    } catch (err) {
      console.error('Failed to join room:', err);
    }
  }, [noteId, enabled]);

  // Leave collaboration room
  const leaveRoom = useCallback(async () => {
    if (!noteId) return;

    try {
      await collaborationService.leaveRoom(noteId);
      setIsConnected(false);
      setParticipants([]);
    } catch (err) {
      console.error('Failed to leave room:', err);
    }
  }, [noteId]);

  // Invite collaborator
  const inviteCollaborator = useCallback(
    async (email: string, role: CollaboratorRole, message?: string, inviterName?: string, noteTitle?: string) => {
      if (!noteId) return;

      try {
        await collaborationService.inviteCollaborator(noteId, {
          invitee_email: email,
          role,
          message,
          inviter_name: inviterName,
          note_title: noteTitle,
        });
        await fetchData();
      } catch (err) {
        throw err;
      }
    },
    [noteId, fetchData]
  );

  // Remove collaborator
  const removeCollaborator = useCallback(
    async (collaboratorId: string) => {
      if (!noteId) return;

      try {
        await collaborationService.removeCollaborator(noteId, collaboratorId);
        setCollaborators((prev) => prev.filter((c) => c.id !== collaboratorId));
      } catch (err) {
        throw err;
      }
    },
    [noteId]
  );

  // Update collaborator role
  const updateRole = useCallback(
    async (collaboratorId: string, role: CollaboratorRole) => {
      if (!noteId) return;

      try {
        const updated = await collaborationService.updateCollaborator(
          noteId,
          collaboratorId,
          { role }
        );
        setCollaborators((prev) =>
          prev.map((c) => (c.id === collaboratorId ? updated : c))
        );
      } catch (err) {
        throw err;
      }
    },
    [noteId]
  );

  // Update visibility
  const updateVisibility = useCallback(
    async (visibility: NoteVisibility) => {
      if (!noteId) return;

      try {
        const updated = await collaborationService.updateVisibility(noteId, { visibility });
        setShareSettings(updated);
      } catch (err) {
        throw err;
      }
    },
    [noteId]
  );

  // Throttled cursor update
  const updateCursor = useCallback(
    (line: number, column: number, selectionStart?: number, selectionEnd?: number) => {
      if (!noteId || !isConnected) return;

      if (cursorThrottleRef.current) {
        clearTimeout(cursorThrottleRef.current);
      }

      cursorThrottleRef.current = setTimeout(() => {
        collaborationService.updateCursor(noteId, {
          line,
          column,
          selection_start: selectionStart,
          selection_end: selectionEnd,
        });
      }, 50); // Throttle to 50ms
    },
    [noteId, isConnected]
  );

  // Throttled content broadcast
  const broadcastContent = useCallback(
    (content: unknown, version: number) => {
      if (!noteId || !isConnected) return;

      if (contentThrottleRef.current) {
        clearTimeout(contentThrottleRef.current);
      }

      contentThrottleRef.current = setTimeout(() => {
        collaborationService.broadcastContent(noteId, content, version);
      }, 100); // Throttle to 100ms
    },
    [noteId, isConnected]
  );

  // Initial data fetch
  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (cursorThrottleRef.current) clearTimeout(cursorThrottleRef.current);
      if (contentThrottleRef.current) clearTimeout(contentThrottleRef.current);
      if (wsRef.current) wsRef.current.close();
    };
  }, []);

  return {
    collaborators,
    participants,
    shareSettings,
    isLoading,
    error,
    isConnected,
    inviteCollaborator,
    removeCollaborator,
    updateRole,
    updateVisibility,
    updateCursor,
    broadcastContent,
    joinRoom,
    leaveRoom,
    refresh: fetchData,
  };
}

// Hook for managing invitations
export function useInvitations() {
  const [invitations, setInvitations] = useState<NoteInvitation[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchInvitations = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const data = await collaborationService.getMyInvitations();
      setInvitations(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load invitations');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const acceptInvitation = useCallback(async (token: string, userName?: string) => {
    try {
      await collaborationService.acceptInvitation({ token, user_name: userName });
      await fetchInvitations();
    } catch (err) {
      throw err;
    }
  }, [fetchInvitations]);

  const declineInvitation = useCallback(async (invitationId: string) => {
    try {
      await collaborationService.declineInvitation(invitationId);
      setInvitations((prev) => prev.filter((i) => i.id !== invitationId));
    } catch (err) {
      throw err;
    }
  }, []);

  useEffect(() => {
    fetchInvitations();
  }, [fetchInvitations]);

  return {
    invitations,
    isLoading,
    error,
    acceptInvitation,
    declineInvitation,
    refresh: fetchInvitations,
  };
}
