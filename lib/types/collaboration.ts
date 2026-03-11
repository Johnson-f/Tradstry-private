// Collaboration types for notebook sharing and real-time editing

export type CollaboratorRole = 'owner' | 'editor' | 'viewer';
export type InvitationStatus = 'pending' | 'accepted' | 'declined' | 'expired' | 'revoked';
export type NoteVisibility = 'private' | 'shared' | 'public';
export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error';

export interface NoteCollaborator {
  id: string;
  note_id: string;
  user_id: string;
  user_email: string;
  user_name?: string;
  role: CollaboratorRole;
  invited_by: string;
  joined_at: string;
  last_seen_at?: string;
  created_at: string;
  updated_at: string;
}

export interface NoteInvitation {
  id: string;
  note_id: string;
  inviter_id: string;
  inviter_email: string;
  invitee_email: string;
  invitee_user_id?: string;
  role: CollaboratorRole;
  token: string;
  status: InvitationStatus;
  message?: string;
  expires_at: string;
  accepted_at?: string;
  created_at: string;
  updated_at: string;
}

export interface NoteShareSettings {
  id: string;
  note_id: string;
  owner_id: string;
  visibility: NoteVisibility;
  public_slug?: string;
  allow_comments: boolean;
  allow_copy: boolean;
  view_count: number;
  created_at: string;
  updated_at: string;
}

export interface CursorPosition {
  user_id: string;
  user_name: string;
  user_color: string;
  line: number;
  column: number;
  selection_start?: number;
  selection_end?: number;
}

export interface CollaboratorPresence {
  user_id: string;
  user_email: string;
  user_name?: string;
  role: CollaboratorRole;
  cursor?: CursorPosition;
  is_active: boolean;
  joined_at: string;
  last_activity: string;
}

export interface NoteRoom {
  note_id: string;
  participants: CollaboratorPresence[];
  created_at: string;
}

// WebSocket event types for collaboration
export type CollaborationEventType =
  | 'collaborator_joined'
  | 'collaborator_left'
  | 'collaborator_cursor_move'
  | 'collaborator_selection'
  | 'note_content_update'
  | 'note_invitation_received'
  | 'note_invitation_accepted'
  | 'note_invitation_declined'
  | 'note_visibility_changed';

export interface CollaborationEvent {
  event: CollaborationEventType;
  data: unknown;
  timestamp: string;
}

// ==================== API Request Types ====================

export interface InviteCollaboratorRequest {
  invitee_email: string;
  role: CollaboratorRole;
  message?: string;
  inviter_name?: string;
  note_title?: string;
}

export interface AcceptInvitationRequest {
  token: string;
  user_name?: string;
  note_title?: string;
}

export interface UpdateVisibilityRequest {
  visibility: NoteVisibility;
  allow_comments?: boolean;
  allow_copy?: boolean;
}

export interface JoinRoomRequest {
  user_name?: string;
}

export interface CursorUpdateRequest {
  line: number;
  column: number;
  selection_start?: number;
  selection_end?: number;
}

export interface ContentUpdateRequest {
  content: unknown;
  version: number;
}

export interface UpdateCollaboratorRequest {
  role?: CollaboratorRole;
  user_name?: string;
}

// ==================== API Response Types ====================

export interface ApiResponse<T> {
  success: boolean;
  message: string;
  data?: T;
}

export type CollaboratorsResponse = ApiResponse<NoteCollaborator[]>;
export type InvitationResponse = ApiResponse<NoteInvitation>;
export type InvitationsResponse = ApiResponse<NoteInvitation[]>;
export type ShareSettingsResponse = ApiResponse<NoteShareSettings>;
export type ParticipantsResponse = ApiResponse<CollaboratorPresence[]>;
export type RoomResponse = ApiResponse<NoteRoom>;

// ==================== WebSocket Payload Types ====================

export interface ContentUpdatePayload {
  note_id: string;
  sender_id: string;
  content: unknown;
  version: number;
}

export interface CursorUpdatePayload {
  note_id: string;
  cursor: CursorPosition;
}

export interface CollaboratorJoinedPayload {
  note_id: string;
  user_id: string;
  user_email: string;
  user_name?: string;
  color: string;
}

export interface CollaboratorLeftPayload {
  note_id: string;
  user_id: string;
}

export interface InvitationReceivedPayload {
  invitation_id: string;
  note_id: string;
  inviter_email: string;
  role: CollaboratorRole;
  message?: string;
}

export interface InvitationAcceptedPayload {
  invitation_id: string;
  note_id: string;
  user_id: string;
  user_email: string;
}

export interface VisibilityChangedPayload {
  note_id: string;
  visibility: NoteVisibility;
  public_slug?: string;
}
