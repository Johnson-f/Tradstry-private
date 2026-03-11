// Notebook Note types
export interface NotebookNote {
  id: string;
  title: string;
  content: Record<string, unknown>; // Lexical editor JSON state
  content_plain_text: string | null;
  word_count: number;
  is_pinned: boolean;
  is_archived: boolean;
  is_deleted: boolean;
  folder_id: string | null;
  tags: string[] | null; // Array of tag IDs
  created_at: string;
  updated_at: string;
  last_synced_at: string | null;
}

export interface CreateNoteRequest {
  title?: string;
  content?: Record<string, unknown>;
  content_plain_text?: string;
  folder_id?: string;
  tags?: string[];
}

export interface UpdateNoteRequest {
  title?: string;
  content?: Record<string, unknown>;
  content_plain_text?: string;
  is_pinned?: boolean;
  is_archived?: boolean;
  is_deleted?: boolean;
  folder_id?: string | null;
  tags?: string[];
}

export interface NoteQuery {
  folder_id?: string;
  is_pinned?: boolean;
  is_archived?: boolean;
  is_deleted?: boolean;
  search?: string;
  limit?: number;
  offset?: number;
}

// Notebook Folder types
export interface NotebookFolder {
  id: string;
  name: string;
  is_favorite: boolean;
  parent_folder_id: string | null;
  position: number;
  created_at: string;
  updated_at: string;
}

export interface CreateFolderRequest {
  name: string;
  is_favorite?: boolean;
  parent_folder_id?: string;
  position?: number;
}

export interface UpdateFolderRequest {
  name?: string;
  is_favorite?: boolean;
  parent_folder_id?: string | null;
  position?: number;
}

// Notebook Tag types
export interface NotebookTag {
  id: string;
  name: string;
  color: string | null;
  is_favorite: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateTagRequest {
  name: string;
  color?: string;
  is_favorite?: boolean;
}

export interface UpdateTagRequest {
  name?: string;
  color?: string;
  is_favorite?: boolean;
}

// Notebook Image types
export interface NotebookImage {
  id: string;
  note_id: string;
  src: string;
  storage_path: string | null;
  alt_text: string | null;
  caption: string | null;
  width: number | null;
  height: number | null;
  position: number;
  created_at: string;
  updated_at: string;
}

export interface CreateImageRequest {
  note_id: string;
  src: string;
  storage_path?: string;
  alt_text?: string;
  caption?: string;
  width?: number;
  height?: number;
  position?: number;
}

export interface UpdateImageRequest {
  src?: string;
  alt_text?: string;
  caption?: string;
  width?: number;
  height?: number;
  position?: number;
}

// Image upload types
export interface ImageUploadRequest {
  note_id: string;
  file_name: string;
  content_type: string;
  file_data: string; // Base64 encoded image data
  alt_text?: string;
  caption?: string;
  width?: number;
  height?: number;
  position?: number;
}

// Backend returns NotebookImage directly from upload endpoint
export interface ImageUploadResponse {
  image: NotebookImage;
  public_url: string;
}

// API Response wrapper
export interface NotebookApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string;
}

// Note with tags (joined response)
export interface NotebookNoteWithTags extends NotebookNote {
  tag_objects?: NotebookTag[];
}

// Folder tree structure for UI
export interface FolderTreeNode extends NotebookFolder {
  children: FolderTreeNode[];
  notes_count?: number;
}

// Set tags request
export interface SetNoteTagsRequest {
  tag_ids: string[];
}

// ==================== Calendar Types ====================

// Calendar Provider enum
export type CalendarProvider = "google" | "microsoft";

// Event Source enum
export type EventSource = "local" | "google" | "microsoft";

// Event Status enum
export type EventStatus = "confirmed" | "tentative" | "cancelled";

// Calendar Connection types
export interface CalendarConnection {
  id: string;
  provider: CalendarProvider;
  access_token: string;
  refresh_token: string;
  token_expires_at: string;
  account_email: string;
  account_name: string | null;
  enabled: boolean;
  last_sync_at: string | null;
  sync_error: string | null;
  created_at: string;
  updated_at: string;
}

// Calendar Event types
export interface CalendarEvent {
  id: string;
  title: string;
  description: string | null;
  start_time: string;
  end_time: string | null;
  all_day: boolean;
  reminder_minutes: number | null;
  recurrence_rule: string | null;
  source: EventSource;
  external_id: string | null;
  external_calendar_id: string | null;
  connection_id: string | null;
  color: string | null;
  location: string | null;
  status: EventStatus;
  is_reminder: boolean;
  reminder_sent: boolean;
  metadata: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
  synced_at: string | null;
}

// Request types
export interface CreateCalendarEventRequest {
  title: string;
  description?: string;
  start_time: string;
  end_time?: string;
  all_day?: boolean;
  reminder_minutes?: number;
  recurrence_rule?: string;
  color?: string;
  location?: string;
  is_reminder?: boolean;
  metadata?: Record<string, unknown>;
}

export interface UpdateCalendarEventRequest {
  title?: string;
  description?: string | null;
  start_time?: string;
  end_time?: string | null;
  all_day?: boolean;
  reminder_minutes?: number | null;
  recurrence_rule?: string | null;
  color?: string | null;
  location?: string | null;
  status?: EventStatus;
  is_reminder?: boolean;
  reminder_sent?: boolean;
  metadata?: Record<string, unknown> | null;
}

export interface CalendarEventQuery {
  start_time?: string;
  end_time?: string;
  source?: EventSource;
  connection_id?: string;
  is_reminder?: boolean;
  limit?: number;
  offset?: number;
}

export interface OAuthCallbackRequest {
  code: string;
  state?: string;
}

export interface ToggleConnectionRequest {
  enabled: boolean;
}

// Response types
export interface AuthUrlResponse {
  success: boolean;
  url: string | null;
  message: string;
}

export interface CalendarConnectionResponse {
  success: boolean;
  message: string;
  data: CalendarConnection | null;
}

export interface CalendarConnectionListResponse {
  success: boolean;
  message: string;
  data: CalendarConnection[] | null;
}

export interface CalendarEventResponse {
  success: boolean;
  message: string;
  data: CalendarEvent | null;
}

export interface CalendarEventListResponse {
  success: boolean;
  message: string;
  data: CalendarEvent[] | null;
}

export interface CalendarSyncResponse {
  success: boolean;
  message: string;
  synced_connections: number;
  failed_connections: number;
  synced_events: number;
}
