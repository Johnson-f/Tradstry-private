"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  notebookNotesService,
  notebookFoldersService,
  notebookTagsService,
  notebookImagesService,
  notebookCalendarService,
} from "@/lib/services/notebook-service";
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
  // Calendar types
  CalendarConnection,
  CalendarEvent,
  CreateCalendarEventRequest,
  UpdateCalendarEventRequest,
  CalendarEventQuery,
  CalendarSyncResponse,
} from "@/lib/types/notebook";

// Query keys
export const notebookKeys = {
  all: ["notebook"] as const,
  notes: () => [...notebookKeys.all, "notes"] as const,
  notesList: (query?: NoteQuery) => [...notebookKeys.notes(), "list", query] as const,
  note: (id: string) => [...notebookKeys.notes(), id] as const,
  noteImages: (noteId: string) => [...notebookKeys.note(noteId), "images"] as const,
  noteTags: (noteId: string) => [...notebookKeys.note(noteId), "tags"] as const,
  trash: () => [...notebookKeys.notes(), "trash"] as const,
  folders: () => [...notebookKeys.all, "folders"] as const,
  foldersList: () => [...notebookKeys.folders(), "list"] as const,
  folder: (id: string) => [...notebookKeys.folders(), id] as const,
  foldersFavorites: () => [...notebookKeys.folders(), "favorites"] as const,
  tags: () => [...notebookKeys.all, "tags"] as const,
  tagsList: () => [...notebookKeys.tags(), "list"] as const,
  tag: (id: string) => [...notebookKeys.tags(), id] as const,
  images: () => [...notebookKeys.all, "images"] as const,
  image: (id: string) => [...notebookKeys.images(), id] as const,
  // Calendar keys
  calendar: () => [...notebookKeys.all, "calendar"] as const,
  calendarConnections: () => [...notebookKeys.calendar(), "connections"] as const,
  calendarEvents: () => [...notebookKeys.calendar(), "events"] as const,
  calendarEventsList: (query?: CalendarEventQuery) => [...notebookKeys.calendarEvents(), "list", query] as const,
  calendarEvent: (id: string) => [...notebookKeys.calendarEvents(), id] as const,
};

// ============================================================================
// Notes Hooks
// ============================================================================
export function useNotebookNotes(query?: NoteQuery) {
  return useQuery<NotebookNote[]>({
    queryKey: notebookKeys.notesList(query),
    queryFn: async () => {
      const res = await notebookNotesService.list(query);
      if (!res.success) throw new Error(res.message || "Failed to fetch notes");
      return res.data ?? [];
    },
    staleTime: 5 * 60 * 1000,
  });
}

export function useNotebookNote(id: string) {
  return useQuery<NotebookNote>({
    queryKey: notebookKeys.note(id),
    queryFn: async () => {
      const res = await notebookNotesService.getById(id);
      if (!res.success) throw new Error(res.message || "Failed to fetch note");
      return res.data!;
    },
    enabled: !!id,
    staleTime: 5 * 60 * 1000,
  });
}

export function useNotebookTrash() {
  return useQuery<NotebookNote[]>({
    queryKey: notebookKeys.trash(),
    queryFn: async () => {
      const res = await notebookNotesService.getTrash();
      if (!res.success) throw new Error(res.message || "Failed to fetch trash");
      return res.data ?? [];
    },
    staleTime: 5 * 60 * 1000,
  });
}

export function useCreateNote() {
  const queryClient = useQueryClient();

  return useMutation<NotebookNote, Error, CreateNoteRequest>({
    mutationFn: async (payload) => {
      const res = await notebookNotesService.create(payload);
      if (!res.success) throw new Error(res.message || "Failed to create note");
      return res.data!;
    },
    onSuccess: (newNote) => {
      queryClient.setQueryData<NotebookNote[]>(
        notebookKeys.notesList(),
        (prev) => (prev ? [newNote, ...prev] : [newNote])
      );
      queryClient.invalidateQueries({ queryKey: notebookKeys.notes() });
    },
  });
}

export function useUpdateNote() {
  const queryClient = useQueryClient();

  return useMutation<NotebookNote, Error, { id: string; payload: UpdateNoteRequest }>({
    mutationFn: async ({ id, payload }) => {
      const res = await notebookNotesService.update(id, payload);
      if (!res.success) throw new Error(res.message || "Failed to update note");
      return res.data!;
    },
    onSuccess: (updatedNote) => {
      queryClient.setQueryData(notebookKeys.note(updatedNote.id), updatedNote);
      queryClient.setQueryData<NotebookNote[]>(
        notebookKeys.notesList(),
        (prev) => prev?.map((n) => (n.id === updatedNote.id ? updatedNote : n)) ?? []
      );
    },
  });
}

export function useDeleteNote() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookNotesService.delete(id);
      if (!res.success) throw new Error(res.message || "Failed to delete note");
      return true;
    },
    onSuccess: (_, id) => {
      queryClient.setQueryData<NotebookNote[]>(
        notebookKeys.notesList(),
        (prev) => prev?.filter((n) => n.id !== id) ?? []
      );
      queryClient.invalidateQueries({ queryKey: notebookKeys.trash() });
    },
  });
}

export function useRestoreNote() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookNotesService.restore(id);
      if (!res.success) throw new Error(res.message || "Failed to restore note");
      return true;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.notes() });
      queryClient.invalidateQueries({ queryKey: notebookKeys.trash() });
    },
  });
}

export function usePermanentDeleteNote() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookNotesService.permanentDelete(id);
      if (!res.success) throw new Error(res.message || "Failed to permanently delete note");
      return true;
    },
    onSuccess: (_, id) => {
      queryClient.setQueryData<NotebookNote[]>(
        notebookKeys.trash(),
        (prev) => prev?.filter((n) => n.id !== id) ?? []
      );
    },
  });
}

// Note Tags
export function useNoteTags(noteId: string) {
  return useQuery<NotebookTag[]>({
    queryKey: notebookKeys.noteTags(noteId),
    queryFn: async () => {
      const res = await notebookNotesService.getTags(noteId);
      if (!res.success) throw new Error(res.message || "Failed to fetch note tags");
      return res.data ?? [];
    },
    enabled: !!noteId,
    staleTime: 5 * 60 * 1000,
  });
}

export function useSetNoteTags() {
  const queryClient = useQueryClient();

  return useMutation<NotebookTag[], Error, { noteId: string; tagIds: string[] }>({
    mutationFn: async ({ noteId, tagIds }) => {
      const res = await notebookNotesService.setTags(noteId, tagIds);
      if (!res.success) throw new Error(res.message || "Failed to set note tags");
      return res.data ?? [];
    },
    onSuccess: (tags, { noteId }) => {
      queryClient.setQueryData(notebookKeys.noteTags(noteId), tags);
    },
  });
}

// Note Images
export function useNoteImages(noteId: string) {
  return useQuery<NotebookImage[]>({
    queryKey: notebookKeys.noteImages(noteId),
    queryFn: async () => {
      const res = await notebookNotesService.getImages(noteId);
      if (!res.success) throw new Error(res.message || "Failed to fetch note images");
      return res.data ?? [];
    },
    enabled: !!noteId,
    staleTime: 5 * 60 * 1000,
  });
}

// ============================================================================
// Folders Hooks
// ============================================================================
export function useNotebookFolders() {
  return useQuery<NotebookFolder[]>({
    queryKey: notebookKeys.foldersList(),
    queryFn: async () => {
      const res = await notebookFoldersService.list();
      if (!res.success) throw new Error(res.message || "Failed to fetch folders");
      return res.data ?? [];
    },
    staleTime: 5 * 60 * 1000,
  });
}

export function useNotebookFolder(id: string) {
  return useQuery<NotebookFolder>({
    queryKey: notebookKeys.folder(id),
    queryFn: async () => {
      const res = await notebookFoldersService.getById(id);
      if (!res.success) throw new Error(res.message || "Failed to fetch folder");
      return res.data!;
    },
    enabled: !!id,
    staleTime: 5 * 60 * 1000,
  });
}

export function useFavoriteFolders() {
  return useQuery<NotebookFolder[]>({
    queryKey: notebookKeys.foldersFavorites(),
    queryFn: async () => {
      const res = await notebookFoldersService.getFavorites();
      if (!res.success) throw new Error(res.message || "Failed to fetch favorite folders");
      return res.data ?? [];
    },
    staleTime: 5 * 60 * 1000,
  });
}

export function useCreateFolder() {
  const queryClient = useQueryClient();

  return useMutation<NotebookFolder, Error, CreateFolderRequest>({
    mutationFn: async (payload) => {
      const res = await notebookFoldersService.create(payload);
      if (!res.success) throw new Error(res.message || "Failed to create folder");
      return res.data!;
    },
    onSuccess: (newFolder) => {
      queryClient.setQueryData<NotebookFolder[]>(
        notebookKeys.foldersList(),
        (prev) => (prev ? [...prev, newFolder] : [newFolder])
      );
      if (newFolder.is_favorite) {
        queryClient.invalidateQueries({ queryKey: notebookKeys.foldersFavorites() });
      }
    },
  });
}

export function useUpdateFolder() {
  const queryClient = useQueryClient();

  return useMutation<NotebookFolder, Error, { id: string; payload: UpdateFolderRequest }>({
    mutationFn: async ({ id, payload }) => {
      const res = await notebookFoldersService.update(id, payload);
      if (!res.success) throw new Error(res.message || "Failed to update folder");
      return res.data!;
    },
    onSuccess: (updatedFolder) => {
      queryClient.setQueryData(notebookKeys.folder(updatedFolder.id), updatedFolder);
      queryClient.setQueryData<NotebookFolder[]>(
        notebookKeys.foldersList(),
        (prev) => prev?.map((f) => (f.id === updatedFolder.id ? updatedFolder : f)) ?? []
      );
      queryClient.invalidateQueries({ queryKey: notebookKeys.foldersFavorites() });
    },
  });
}

export function useDeleteFolder() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookFoldersService.delete(id);
      if (!res.success) throw new Error(res.message || "Failed to delete folder");
      return true;
    },
    onSuccess: (_, id) => {
      queryClient.setQueryData<NotebookFolder[]>(
        notebookKeys.foldersList(),
        (prev) => prev?.filter((f) => f.id !== id) ?? []
      );
      queryClient.invalidateQueries({ queryKey: notebookKeys.foldersFavorites() });
      // Notes in this folder are cascade deleted, so invalidate notes
      queryClient.invalidateQueries({ queryKey: notebookKeys.notes() });
    },
  });
}

// ============================================================================
// Tags Hooks
// ============================================================================
export function useNotebookTags() {
  return useQuery<NotebookTag[]>({
    queryKey: notebookKeys.tagsList(),
    queryFn: async () => {
      const res = await notebookTagsService.list();
      if (!res.success) throw new Error(res.message || "Failed to fetch tags");
      return res.data ?? [];
    },
    staleTime: 5 * 60 * 1000,
  });
}

export function useNotebookTag(id: string) {
  return useQuery<NotebookTag>({
    queryKey: notebookKeys.tag(id),
    queryFn: async () => {
      const res = await notebookTagsService.getById(id);
      if (!res.success) throw new Error(res.message || "Failed to fetch tag");
      return res.data!;
    },
    enabled: !!id,
    staleTime: 5 * 60 * 1000,
  });
}

export function useCreateTag() {
  const queryClient = useQueryClient();

  return useMutation<NotebookTag, Error, CreateTagRequest>({
    mutationFn: async (payload) => {
      const res = await notebookTagsService.create(payload);
      if (!res.success) throw new Error(res.message || "Failed to create tag");
      return res.data!;
    },
    onSuccess: (newTag) => {
      queryClient.setQueryData<NotebookTag[]>(
        notebookKeys.tagsList(),
        (prev) => (prev ? [...prev, newTag] : [newTag])
      );
    },
  });
}

export function useUpdateTag() {
  const queryClient = useQueryClient();

  return useMutation<NotebookTag, Error, { id: string; payload: UpdateTagRequest }>({
    mutationFn: async ({ id, payload }) => {
      const res = await notebookTagsService.update(id, payload);
      if (!res.success) throw new Error(res.message || "Failed to update tag");
      return res.data!;
    },
    onSuccess: (updatedTag) => {
      queryClient.setQueryData(notebookKeys.tag(updatedTag.id), updatedTag);
      queryClient.setQueryData<NotebookTag[]>(
        notebookKeys.tagsList(),
        (prev) => prev?.map((t) => (t.id === updatedTag.id ? updatedTag : t)) ?? []
      );
    },
  });
}

export function useDeleteTag() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookTagsService.delete(id);
      if (!res.success) throw new Error(res.message || "Failed to delete tag");
      return true;
    },
    onSuccess: (_, id) => {
      queryClient.setQueryData<NotebookTag[]>(
        notebookKeys.tagsList(),
        (prev) => prev?.filter((t) => t.id !== id) ?? []
      );
    },
  });
}

// ============================================================================
// Images Hooks
// ============================================================================
export function useNotebookImage(id: string) {
  return useQuery<NotebookImage>({
    queryKey: notebookKeys.image(id),
    queryFn: async () => {
      const res = await notebookImagesService.getById(id);
      if (!res.success) throw new Error(res.message || "Failed to fetch image");
      return res.data!;
    },
    enabled: !!id,
    staleTime: 5 * 60 * 1000,
  });
}

export function useCreateImage() {
  const queryClient = useQueryClient();

  return useMutation<NotebookImage, Error, CreateImageRequest>({
    mutationFn: async (payload) => {
      const res = await notebookImagesService.create(payload);
      if (!res.success) throw new Error(res.message || "Failed to create image");
      return res.data!;
    },
    onSuccess: (newImage) => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.noteImages(newImage.note_id) });
    },
  });
}

export function useUploadImage() {
  const queryClient = useQueryClient();

  return useMutation<NotebookImage, Error, { 
    noteId: string; 
    file: File;
    options?: {
      altText?: string;
      caption?: string;
      width?: number;
      height?: number;
      position?: number;
    };
  }>({
    mutationFn: async ({ noteId, file, options }) => {
      const res = await notebookImagesService.uploadFile(noteId, file, options);
      if (!res.success) throw new Error(res.message || "Failed to upload image");
      return res.data!;
    },
    onSuccess: (image) => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.noteImages(image.note_id) });
    },
  });
}

export function useUploadImageBase64() {
  const queryClient = useQueryClient();

  return useMutation<NotebookImage, Error, {
    noteId: string;
    base64Data: string;
    fileName: string;
    contentType: string;
    options?: {
      altText?: string;
      caption?: string;
      width?: number;
      height?: number;
      position?: number;
    };
  }>({
    mutationFn: async ({ noteId, base64Data, fileName, contentType, options }) => {
      const res = await notebookImagesService.uploadBase64(
        noteId, 
        base64Data, 
        fileName, 
        contentType, 
        options
      );
      if (!res.success) throw new Error(res.message || "Failed to upload image");
      return res.data!;
    },
    onSuccess: (image) => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.noteImages(image.note_id) });
    },
  });
}

export function useUpdateImage() {
  const queryClient = useQueryClient();

  return useMutation<NotebookImage, Error, { id: string; payload: UpdateImageRequest }>({
    mutationFn: async ({ id, payload }) => {
      const res = await notebookImagesService.update(id, payload);
      if (!res.success) throw new Error(res.message || "Failed to update image");
      return res.data!;
    },
    onSuccess: (updatedImage) => {
      queryClient.setQueryData(notebookKeys.image(updatedImage.id), updatedImage);
      queryClient.invalidateQueries({ queryKey: notebookKeys.noteImages(updatedImage.note_id) });
    },
  });
}

export function useDeleteImage() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, { id: string; noteId: string }>({
    mutationFn: async ({ id }) => {
      const res = await notebookImagesService.delete(id);
      if (!res.success) throw new Error(res.message || "Failed to delete image");
      return true;
    },
    onSuccess: (_, { noteId }) => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.noteImages(noteId) });
    },
  });
}

// ============================================================================
// Batch Image Upload Hook (for uploading pending base64 images)
// ============================================================================
export function useBatchUploadImages() {
  const queryClient = useQueryClient();

  return useMutation<
    { uploaded: NotebookImage[]; failed: string[] },
    Error,
    {
      noteId: string;
      images: Array<{
        base64Data: string;
        fileName: string;
        contentType: string;
        altText?: string;
        width?: number;
        height?: number;
      }>;
    }
  >({
    mutationFn: async ({ noteId, images }) => {
      const uploaded: NotebookImage[] = [];
      const failed: string[] = [];

      for (const img of images) {
        try {
          const res = await notebookImagesService.uploadBase64(
            noteId,
            img.base64Data,
            img.fileName,
            img.contentType,
            {
              altText: img.altText,
              width: img.width,
              height: img.height,
            }
          );
          if (res.success && res.data) {
            uploaded.push(res.data);
          } else {
            failed.push(img.fileName);
          }
        } catch {
          failed.push(img.fileName);
        }
      }

      return { uploaded, failed };
    },
    onSuccess: (_, { noteId }) => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.noteImages(noteId) });
    },
  });
}

// ============================================================================
// Calendar Hooks
// ============================================================================

// Get Google OAuth URL
export function useCalendarAuthUrl() {
  return useQuery({
    queryKey: [...notebookKeys.calendar(), "authUrl"],
    queryFn: async () => {
      const res = await notebookCalendarService.getAuthUrl();
      if (!res.success) throw new Error(res.message || "Failed to get auth URL");
      return res.url!;
    },
    enabled: false, // Only fetch when explicitly called
    staleTime: 5 * 60 * 1000,
  });
}

// Handle OAuth callback
export function useCalendarOAuthCallback() {
  const queryClient = useQueryClient();

  return useMutation<CalendarConnection, Error, { code: string; state?: string }>({
    mutationFn: async ({ code, state }) => {
      const res = await notebookCalendarService.handleOAuthCallback({ code, state });
      if (!res.success) throw new Error(res.message || "Failed to connect calendar");
      return res.data!;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarConnections() });
    },
  });
}

// List calendar connections
export function useCalendarConnections() {
  return useQuery<CalendarConnection[]>({
    queryKey: notebookKeys.calendarConnections(),
    queryFn: async () => {
      const res = await notebookCalendarService.listConnections();
      if (!res.success) throw new Error(res.message || "Failed to fetch connections");
      return res.data ?? [];
    },
    staleTime: 5 * 60 * 1000,
  });
}

// Toggle connection enabled/disabled
export function useToggleCalendarConnection() {
  const queryClient = useQueryClient();

  return useMutation<CalendarConnection, Error, { id: string; enabled: boolean }>({
    mutationFn: async ({ id, enabled }) => {
      const res = await notebookCalendarService.toggleConnection(id, enabled);
      if (!res.success) throw new Error(res.message || "Failed to toggle connection");
      return res.data!;
    },
    onSuccess: (updatedConnection) => {
      queryClient.setQueryData<CalendarConnection[]>(
        notebookKeys.calendarConnections(),
        (prev) => prev?.map((c) => (c.id === updatedConnection.id ? updatedConnection : c)) ?? []
      );
    },
  });
}

// Delete calendar connection
export function useDeleteCalendarConnection() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookCalendarService.deleteConnection(id);
      if (!res.success) throw new Error(res.message || "Failed to delete connection");
      return true;
    },
    onSuccess: (_, id) => {
      queryClient.setQueryData<CalendarConnection[]>(
        notebookKeys.calendarConnections(),
        (prev) => prev?.filter((c) => c.id !== id) ?? []
      );
      // Also invalidate events since they may have been deleted
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarEvents() });
    },
  });
}

// Sync all connections
export function useSyncAllCalendars() {
  const queryClient = useQueryClient();

  return useMutation<CalendarSyncResponse, Error, void>({
    mutationFn: async () => {
      const res = await notebookCalendarService.syncAll();
      if (!res.success) throw new Error(res.message || "Failed to sync calendars");
      return res;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarConnections() });
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarEvents() });
    },
  });
}

// Sync specific connection
export function useSyncCalendarConnection() {
  const queryClient = useQueryClient();

  return useMutation<CalendarSyncResponse, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookCalendarService.syncConnection(id);
      if (!res.success) throw new Error(res.message || "Failed to sync connection");
      return res;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarConnections() });
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarEvents() });
    },
  });
}

// List calendar events
export function useCalendarEvents(query?: CalendarEventQuery) {
  return useQuery<CalendarEvent[]>({
    queryKey: notebookKeys.calendarEventsList(query),
    queryFn: async () => {
      const res = await notebookCalendarService.listEvents(query);
      if (!res.success) throw new Error(res.message || "Failed to fetch events");
      return res.data ?? [];
    },
    staleTime: 2 * 60 * 1000, // 2 minutes for events
  });
}

// Get single calendar event
export function useCalendarEvent(id: string) {
  return useQuery<CalendarEvent>({
    queryKey: notebookKeys.calendarEvent(id),
    queryFn: async () => {
      const res = await notebookCalendarService.getEvent(id);
      if (!res.success) throw new Error(res.message || "Failed to fetch event");
      return res.data!;
    },
    enabled: !!id,
    staleTime: 5 * 60 * 1000,
  });
}

// Create calendar event
export function useCreateCalendarEvent() {
  const queryClient = useQueryClient();

  return useMutation<CalendarEvent, Error, CreateCalendarEventRequest>({
    mutationFn: async (payload) => {
      const res = await notebookCalendarService.createEvent(payload);
      if (!res.success) throw new Error(res.message || "Failed to create event");
      return res.data!;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarEvents() });
    },
  });
}

// Update calendar event
export function useUpdateCalendarEvent() {
  const queryClient = useQueryClient();

  return useMutation<CalendarEvent, Error, { id: string; payload: UpdateCalendarEventRequest }>({
    mutationFn: async ({ id, payload }) => {
      const res = await notebookCalendarService.updateEvent(id, payload);
      if (!res.success) throw new Error(res.message || "Failed to update event");
      return res.data!;
    },
    onSuccess: (updatedEvent) => {
      queryClient.setQueryData(notebookKeys.calendarEvent(updatedEvent.id), updatedEvent);
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarEvents() });
    },
  });
}

// Delete calendar event
export function useDeleteCalendarEvent() {
  const queryClient = useQueryClient();

  return useMutation<boolean, Error, string>({
    mutationFn: async (id) => {
      const res = await notebookCalendarService.deleteEvent(id);
      if (!res.success) throw new Error(res.message || "Failed to delete event");
      return true;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarEvents() });
    },
  });
}

// Create reminder (convenience hook)
export function useCreateReminder() {
  const queryClient = useQueryClient();

  return useMutation<
    CalendarEvent,
    Error,
    {
      title: string;
      startTime: Date;
      reminderMinutes?: number;
      description?: string;
      color?: string;
      location?: string;
    }
  >({
    mutationFn: async ({ title, startTime, reminderMinutes, description, color, location }) => {
      const res = await notebookCalendarService.createReminder(title, startTime, reminderMinutes, {
        description,
        color,
        location,
      });
      if (!res.success) throw new Error(res.message || "Failed to create reminder");
      return res.data!;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: notebookKeys.calendarEvents() });
    },
  });
}

// Get events in date range (convenience hook)
export function useCalendarEventsInRange(
  startDate: Date,
  endDate: Date,
  options?: {
    source?: "local" | "google" | "microsoft";
    isReminder?: boolean;
    limit?: number;
  }
) {
  return useQuery<CalendarEvent[]>({
    queryKey: [
      ...notebookKeys.calendarEvents(),
      "range",
      startDate.toISOString(),
      endDate.toISOString(),
      options,
    ],
    queryFn: async () => {
      const res = await notebookCalendarService.getEventsInRange(startDate, endDate, options);
      if (!res.success) throw new Error(res.message || "Failed to fetch events");
      return res.data ?? [];
    },
    staleTime: 2 * 60 * 1000,
  });
}

// ============================================================================
// Combined Hook for convenience
// ============================================================================
export function useNotebook() {
  const notes = useNotebookNotes();
  const folders = useNotebookFolders();
  const tags = useNotebookTags();
  const trash = useNotebookTrash();

  return {
    notes,
    folders,
    tags,
    trash,
    isLoading: notes.isLoading || folders.isLoading || tags.isLoading,
    isError: notes.isError || folders.isError || tags.isError,
  };
}

// Combined calendar hook
export function useCalendar() {
  const connections = useCalendarConnections();
  const events = useCalendarEvents();

  return {
    connections,
    events,
    isLoading: connections.isLoading || events.isLoading,
    isError: connections.isError || events.isError,
  };
}
