import { apiClient } from "@/lib/services/api-client";
import { apiConfig } from "@/lib/config/api";
import type {
  NotebookNote,
  NotebookFolder,
  NotebookTag,
  NotebookImage,
  CreateNoteRequest,
  UpdateNoteRequest,
  NoteQuery,
  CreateFolderRequest,
  UpdateFolderRequest,
  CreateTagRequest,
  UpdateTagRequest,
  CreateImageRequest,
  UpdateImageRequest,
  ImageUploadRequest,
  ImageUploadResponse,
  NotebookApiResponse,
  SetNoteTagsRequest,
  // Calendar types
  CalendarConnection,
  CalendarEvent,
  CreateCalendarEventRequest,
  UpdateCalendarEventRequest,
  CalendarEventQuery,
  OAuthCallbackRequest,
  ToggleConnectionRequest,
  AuthUrlResponse,
  CalendarConnectionResponse,
  CalendarConnectionListResponse,
  CalendarEventResponse,
  CalendarEventListResponse,
  CalendarSyncResponse,
} from "@/lib/types/notebook";

// ============================================================================
// Notes Service
// ============================================================================
class NotebookNotesService {
  async list(
    query?: NoteQuery
  ): Promise<NotebookApiResponse<NotebookNote[]>> {
    return apiClient.get(apiConfig.endpoints.notebook.notes.base, {
      params: query,
    });
  }

  async getById(id: string): Promise<NotebookApiResponse<NotebookNote>> {
    return apiClient.get(apiConfig.endpoints.notebook.notes.byId(id));
  }

  async create(
    payload: CreateNoteRequest
  ): Promise<NotebookApiResponse<NotebookNote>> {
    return apiClient.post(apiConfig.endpoints.notebook.notes.base, payload);
  }

  async update(
    id: string,
    payload: UpdateNoteRequest
  ): Promise<NotebookApiResponse<NotebookNote>> {
    return apiClient.put(apiConfig.endpoints.notebook.notes.byId(id), payload);
  }

  async delete(id: string): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(apiConfig.endpoints.notebook.notes.byId(id));
  }

  async getTrash(): Promise<NotebookApiResponse<NotebookNote[]>> {
    return apiClient.get(apiConfig.endpoints.notebook.notes.trash);
  }

  async restore(id: string): Promise<NotebookApiResponse<boolean>> {
    return apiClient.post(apiConfig.endpoints.notebook.notes.restore(id), {});
  }

  async permanentDelete(id: string): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(apiConfig.endpoints.notebook.notes.permanent(id));
  }

  // Note tags
  async getTags(noteId: string): Promise<NotebookApiResponse<NotebookTag[]>> {
    return apiClient.get(apiConfig.endpoints.notebook.notes.tags(noteId));
  }

  async setTags(
    noteId: string,
    tagIds: string[]
  ): Promise<NotebookApiResponse<NotebookTag[]>> {
    const payload: SetNoteTagsRequest = { tag_ids: tagIds };
    return apiClient.put(apiConfig.endpoints.notebook.notes.tags(noteId), payload);
  }

  async addTag(
    noteId: string,
    tagId: string
  ): Promise<NotebookApiResponse<boolean>> {
    return apiClient.post(
      apiConfig.endpoints.notebook.notes.tag(noteId, tagId),
      {}
    );
  }

  async removeTag(
    noteId: string,
    tagId: string
  ): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(
      apiConfig.endpoints.notebook.notes.tag(noteId, tagId)
    );
  }

  // Note images
  async getImages(
    noteId: string
  ): Promise<NotebookApiResponse<NotebookImage[]>> {
    return apiClient.get(apiConfig.endpoints.notebook.notes.images(noteId));
  }
}

// ============================================================================
// Folders Service
// ============================================================================
class NotebookFoldersService {
  async list(): Promise<NotebookApiResponse<NotebookFolder[]>> {
    return apiClient.get(apiConfig.endpoints.notebook.folders.base);
  }

  async getById(id: string): Promise<NotebookApiResponse<NotebookFolder>> {
    return apiClient.get(apiConfig.endpoints.notebook.folders.byId(id));
  }

  async create(
    payload: CreateFolderRequest
  ): Promise<NotebookApiResponse<NotebookFolder>> {
    return apiClient.post(apiConfig.endpoints.notebook.folders.base, payload);
  }

  async update(
    id: string,
    payload: UpdateFolderRequest
  ): Promise<NotebookApiResponse<NotebookFolder>> {
    return apiClient.put(
      apiConfig.endpoints.notebook.folders.byId(id),
      payload
    );
  }

  async delete(id: string): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(apiConfig.endpoints.notebook.folders.byId(id));
  }

  async getFavorites(): Promise<NotebookApiResponse<NotebookFolder[]>> {
    return apiClient.get(apiConfig.endpoints.notebook.folders.favorites);
  }
}

// ============================================================================
// Tags Service
// ============================================================================
class NotebookTagsService {
  async list(): Promise<NotebookApiResponse<NotebookTag[]>> {
    return apiClient.get(apiConfig.endpoints.notebook.tags.base);
  }

  async getById(id: string): Promise<NotebookApiResponse<NotebookTag>> {
    return apiClient.get(apiConfig.endpoints.notebook.tags.byId(id));
  }

  async create(
    payload: CreateTagRequest
  ): Promise<NotebookApiResponse<NotebookTag>> {
    return apiClient.post(apiConfig.endpoints.notebook.tags.base, payload);
  }

  async update(
    id: string,
    payload: UpdateTagRequest
  ): Promise<NotebookApiResponse<NotebookTag>> {
    return apiClient.put(apiConfig.endpoints.notebook.tags.byId(id), payload);
  }

  async delete(id: string): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(apiConfig.endpoints.notebook.tags.byId(id));
  }
}

// ============================================================================
// Images Service
// ============================================================================
class NotebookImagesService {
  async getById(id: string): Promise<NotebookApiResponse<NotebookImage>> {
    return apiClient.get(apiConfig.endpoints.notebook.images.byId(id));
  }

  async create(
    payload: CreateImageRequest
  ): Promise<NotebookApiResponse<NotebookImage>> {
    return apiClient.post(apiConfig.endpoints.notebook.images.base, payload);
  }

  async update(
    id: string,
    payload: UpdateImageRequest
  ): Promise<NotebookApiResponse<NotebookImage>> {
    return apiClient.put(apiConfig.endpoints.notebook.images.byId(id), payload);
  }

  async delete(id: string): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(apiConfig.endpoints.notebook.images.byId(id));
  }

  async upload(
    payload: ImageUploadRequest
  ): Promise<NotebookApiResponse<NotebookImage>> {
    return apiClient.post(apiConfig.endpoints.notebook.images.upload, payload);
  }

  // Helper to upload a File object
  async uploadFile(
    noteId: string,
    file: File,
    options?: {
      altText?: string;
      caption?: string;
      width?: number;
      height?: number;
      position?: number;
    }
  ): Promise<NotebookApiResponse<NotebookImage>> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = async () => {
        try {
          const base64 = (reader.result as string).split(",")[1];
          const result = await this.upload({
            note_id: noteId,
            file_name: file.name,
            content_type: file.type || 'image/png',
            file_data: base64,
            alt_text: options?.altText,
            caption: options?.caption,
            width: options?.width,
            height: options?.height,
            position: options?.position,
          });
          resolve(result);
        } catch (error) {
          reject(error);
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  // Helper to upload from base64 string directly
  async uploadBase64(
    noteId: string,
    base64Data: string,
    fileName: string,
    contentType: string,
    options?: {
      altText?: string;
      caption?: string;
      width?: number;
      height?: number;
      position?: number;
    }
  ): Promise<NotebookApiResponse<NotebookImage>> {
    // Remove data URL prefix if present
    const cleanBase64 = base64Data.includes(',') 
      ? base64Data.split(',')[1] 
      : base64Data;
    
    return this.upload({
      note_id: noteId,
      file_name: fileName,
      content_type: contentType,
      file_data: cleanBase64,
      alt_text: options?.altText,
      caption: options?.caption,
      width: options?.width,
      height: options?.height,
      position: options?.position,
    });
  }
}

// ============================================================================
// Calendar Service
// ============================================================================
class NotebookCalendarService {
  // OAuth
  async getAuthUrl(): Promise<AuthUrlResponse> {
    return apiClient.get(apiConfig.endpoints.notebook.calendar.auth.url);
  }

  async handleOAuthCallback(
    payload: OAuthCallbackRequest
  ): Promise<CalendarConnectionResponse> {
    return apiClient.post(
      apiConfig.endpoints.notebook.calendar.auth.callback,
      payload
    );
  }

  // Connections
  async listConnections(): Promise<CalendarConnectionListResponse> {
    return apiClient.get(apiConfig.endpoints.notebook.calendar.connections.base);
  }

  async toggleConnection(
    id: string,
    enabled: boolean
  ): Promise<CalendarConnectionResponse> {
    const payload: ToggleConnectionRequest = { enabled };
    return apiClient.patch(
      apiConfig.endpoints.notebook.calendar.connections.byId(id),
      payload
    );
  }

  async deleteConnection(
    id: string
  ): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(
      apiConfig.endpoints.notebook.calendar.connections.byId(id)
    );
  }

  // Sync
  async syncAll(): Promise<CalendarSyncResponse> {
    return apiClient.post(apiConfig.endpoints.notebook.calendar.sync.all, {});
  }

  async syncConnection(id: string): Promise<CalendarSyncResponse> {
    return apiClient.post(
      apiConfig.endpoints.notebook.calendar.sync.byId(id),
      {}
    );
  }

  // Events
  async listEvents(
    query?: CalendarEventQuery
  ): Promise<CalendarEventListResponse> {
    const endpoint = query
      ? apiConfig.endpoints.notebook.calendar.events.withQuery(query)
      : apiConfig.endpoints.notebook.calendar.events.base;
    return apiClient.get(endpoint);
  }

  async getEvent(id: string): Promise<CalendarEventResponse> {
    return apiClient.get(apiConfig.endpoints.notebook.calendar.events.byId(id));
  }

  async createEvent(
    payload: CreateCalendarEventRequest
  ): Promise<CalendarEventResponse> {
    return apiClient.post(
      apiConfig.endpoints.notebook.calendar.events.base,
      payload
    );
  }

  async updateEvent(
    id: string,
    payload: UpdateCalendarEventRequest
  ): Promise<CalendarEventResponse> {
    return apiClient.put(
      apiConfig.endpoints.notebook.calendar.events.byId(id),
      payload
    );
  }

  async deleteEvent(id: string): Promise<NotebookApiResponse<boolean>> {
    return apiClient.delete(
      apiConfig.endpoints.notebook.calendar.events.byId(id)
    );
  }

  // Helper: Create a reminder
  async createReminder(
    title: string,
    startTime: Date,
    reminderMinutes?: number,
    options?: {
      description?: string;
      color?: string;
      location?: string;
    }
  ): Promise<CalendarEventResponse> {
    return this.createEvent({
      title,
      start_time: startTime.toISOString(),
      is_reminder: true,
      reminder_minutes: reminderMinutes,
      description: options?.description,
      color: options?.color,
      location: options?.location,
    });
  }

  // Helper: Get events for a date range
  async getEventsInRange(
    startDate: Date,
    endDate: Date,
    options?: {
      source?: "local" | "google" | "microsoft";
      isReminder?: boolean;
      limit?: number;
    }
  ): Promise<CalendarEventListResponse> {
    return this.listEvents({
      start_time: startDate.toISOString(),
      end_time: endDate.toISOString(),
      source: options?.source,
      is_reminder: options?.isReminder,
      limit: options?.limit,
    });
  }
}

// ============================================================================
// Combined Notebook Service
// ============================================================================
class NotebookService {
  notes = new NotebookNotesService();
  folders = new NotebookFoldersService();
  tags = new NotebookTagsService();
  images = new NotebookImagesService();
  calendar = new NotebookCalendarService();
}

export const notebookService = new NotebookService();

// Export individual services for direct access
export const notebookNotesService = notebookService.notes;
export const notebookFoldersService = notebookService.folders;
export const notebookTagsService = notebookService.tags;
export const notebookImagesService = notebookService.images;
export const notebookCalendarService = notebookService.calendar;
