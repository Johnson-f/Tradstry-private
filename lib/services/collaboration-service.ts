import { apiConfig, getFullUrl } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';
import type {
  NoteCollaborator,
  NoteInvitation,
  NoteShareSettings,
  CollaboratorPresence,
  NoteRoom,
  InviteCollaboratorRequest,
  AcceptInvitationRequest,
  UpdateVisibilityRequest,
  UpdateCollaboratorRequest,
} from '@/lib/types/collaboration';

interface ApiResponse<T> {
  success: boolean;
  message: string;
  data: T | null;
}

async function getAuthToken(): Promise<string | null> {
  const supabase = createClient();
  const { data: { session } } = await supabase.auth.getSession();
  return session?.access_token || null;
}

async function fetchWithAuth<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<ApiResponse<T>> {
  const token = await getAuthToken();
  
  const url = getFullUrl(endpoint);
  console.log('[fetchWithAuth] Request:', options.method || 'GET', url);
  console.log('[fetchWithAuth] Has token:', !!token);
  if (options.body) {
    console.log('[fetchWithAuth] Body:', options.body);
  }
  
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      Authorization: token ? `Bearer ${token}` : '',
      ...options.headers,
    },
  });

  console.log('[fetchWithAuth] Response status:', response.status);
  
  if (!response.ok) {
    const errorText = await response.text();
    console.error('[fetchWithAuth] Error response:', errorText);
    let error;
    try {
      error = JSON.parse(errorText);
    } catch {
      error = { message: errorText || 'Request failed' };
    }
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  const data = await response.json();
  console.log('[fetchWithAuth] Response data:', JSON.stringify(data, null, 2));
  return data;
}

export const collaborationService = {
  // ==================== Collaborators ====================
  
  async getCollaborators(noteId: string): Promise<NoteCollaborator[]> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.collaborators(noteId);
    const response = await fetchWithAuth<NoteCollaborator[]>(endpoint);
    return response.data || [];
  },

  async inviteCollaborator(
    noteId: string,
    request: InviteCollaboratorRequest
  ): Promise<NoteInvitation> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.invite(noteId);
    const response = await fetchWithAuth<NoteInvitation>(endpoint, {
      method: 'POST',
      body: JSON.stringify(request),
    });
    if (!response.data) throw new Error('Failed to send invitation');
    return response.data;
  },

  async removeCollaborator(noteId: string, collaboratorId: string): Promise<void> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.collaborator(noteId, collaboratorId);
    await fetchWithAuth(endpoint, { method: 'DELETE' });
  },

  async updateCollaborator(
    noteId: string,
    collaboratorId: string,
    request: UpdateCollaboratorRequest
  ): Promise<NoteCollaborator> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.collaborator(noteId, collaboratorId);
    const response = await fetchWithAuth<NoteCollaborator>(endpoint, {
      method: 'PATCH',
      body: JSON.stringify(request),
    });
    if (!response.data) throw new Error('Failed to update collaborator');
    return response.data;
  },

  // ==================== Invitations ====================

  async getMyInvitations(): Promise<NoteInvitation[]> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.invitations.mine;
    const response = await fetchWithAuth<NoteInvitation[]>(endpoint);
    return response.data || [];
  },

  async acceptInvitation(request: AcceptInvitationRequest): Promise<NoteCollaborator> {
    console.log('[collaborationService.acceptInvitation] Request:', JSON.stringify(request, null, 2));
    const endpoint = apiConfig.endpoints.notebook.collaboration.invitations.accept;
    console.log('[collaborationService.acceptInvitation] Endpoint:', endpoint);
    console.log('[collaborationService.acceptInvitation] Full URL:', getFullUrl(endpoint));
    
    try {
      const response = await fetchWithAuth<NoteCollaborator>(endpoint, {
        method: 'POST',
        body: JSON.stringify(request),
      });
      console.log('[collaborationService.acceptInvitation] Response:', JSON.stringify(response, null, 2));
      if (!response.data) throw new Error('Failed to accept invitation');
      return response.data;
    } catch (error) {
      console.error('[collaborationService.acceptInvitation] Error:', error);
      throw error;
    }
  },

  async declineInvitation(invitationId: string): Promise<void> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.invitations.decline(invitationId);
    await fetchWithAuth(endpoint, { method: 'POST' });
  },

  // ==================== Sharing ====================

  async getShareSettings(noteId: string): Promise<NoteShareSettings | null> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.share(noteId);
    const response = await fetchWithAuth<NoteShareSettings>(endpoint);
    return response.data;
  },

  async updateVisibility(
    noteId: string,
    request: UpdateVisibilityRequest
  ): Promise<NoteShareSettings> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.visibility(noteId);
    const response = await fetchWithAuth<NoteShareSettings>(endpoint, {
      method: 'PUT',
      body: JSON.stringify(request),
    });
    if (!response.data) throw new Error('Failed to update visibility');
    return response.data;
  },

  // ==================== Real-time Collaboration ====================

  async joinRoom(noteId: string, userName?: string): Promise<NoteRoom> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.join(noteId);
    const response = await fetchWithAuth<NoteRoom>(endpoint, {
      method: 'POST',
      body: JSON.stringify({ user_name: userName }),
    });
    if (!response.data) throw new Error('Failed to join room');
    return response.data;
  },

  async leaveRoom(noteId: string): Promise<void> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.leave(noteId);
    await fetchWithAuth(endpoint, { method: 'POST' });
  },

  async updateCursor(
    noteId: string,
    cursor: { line: number; column: number; selection_start?: number; selection_end?: number }
  ): Promise<void> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.cursor(noteId);
    await fetchWithAuth(endpoint, {
      method: 'POST',
      body: JSON.stringify(cursor),
    });
  },

  async broadcastContent(
    noteId: string,
    content: unknown,
    version: number
  ): Promise<void> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.content(noteId);
    await fetchWithAuth(endpoint, {
      method: 'POST',
      body: JSON.stringify({ content, version }),
    });
  },

  async getParticipants(noteId: string): Promise<CollaboratorPresence[]> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.notes.participants(noteId);
    const response = await fetchWithAuth<CollaboratorPresence[]>(endpoint);
    return response.data || [];
  },

  // ==================== Public Notes ====================

  async getPublicNote(slug: string): Promise<unknown> {
    const endpoint = apiConfig.endpoints.notebook.collaboration.public(slug);
    const response = await fetchWithAuth<unknown>(endpoint);
    return response.data;
  },
};

export default collaborationService;
