/**
 * Cache configuration for React Query
 * Centralized cache settings for optimal performance
 */

import { QueryClient } from '@tanstack/react-query';

// Define a generic service interface for type safety
interface NotesService {
  getNote: (id: string) => Promise<unknown>;
}

export const cacheConfig = {
  // Default cache times
  defaultStaleTime: 5 * 60 * 1000, // 5 minutes
  defaultGcTime: 30 * 60 * 1000, // 30 minutes
  
  // Note-specific cache times
  notes: {
    list: {
      staleTime: 5 * 60 * 1000, // 5 minutes
      gcTime: 30 * 60 * 1000, // 30 minutes
    },
    detail: {
      staleTime: 10 * 60 * 1000, // 10 minutes - individual notes stay fresh longer
      gcTime: 60 * 60 * 1000, // 1 hour
    },
    favorites: {
      staleTime: 5 * 60 * 1000, // 5 minutes
      gcTime: 30 * 60 * 1000, // 30 minutes
    },
    trash: {
      staleTime: 2 * 60 * 1000, // 2 minutes - trash changes less frequently
      gcTime: 15 * 60 * 1000, // 15 minutes
    },
  },
  
  // Folder cache times
  folders: {
    staleTime: 10 * 60 * 1000, // 10 minutes - folders change infrequently
    gcTime: 60 * 60 * 1000, // 1 hour
  },
  
  // Tag cache times
  tags: {
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes
  },
  
  // Template cache times
  templates: {
    staleTime: 15 * 60 * 1000, // 15 minutes - templates change rarely
    gcTime: 2 * 60 * 60 * 1000, // 2 hours
  },
  
  // Image cache times
  images: {
    staleTime: 30 * 60 * 1000, // 30 minutes - images rarely change
    gcTime: 2 * 60 * 60 * 1000, // 2 hours
  },
  
  // Global settings
  refetchOnWindowFocus: false,
  refetchOnMount: false,
  refetchOnReconnect: true,
  retry: 3,
  retryDelay: (attemptIndex: number) => Math.min(1000 * 2 ** attemptIndex, 30000),
} as const;

/**
 * Cache invalidation patterns
 */
export const cacheInvalidation = {
  // Invalidate all notes-related queries
  invalidateAllNotes: (queryClient: QueryClient) => {
    queryClient.invalidateQueries({ queryKey: ['notes'] });
  },
  
  // Invalidate specific note
  invalidateNote: (queryClient: QueryClient, noteId: string) => {
    queryClient.invalidateQueries({ queryKey: ['notes', 'detail', noteId] });
  },
  
  // Invalidate notes list
  invalidateNotesList: (queryClient: QueryClient) => {
    queryClient.invalidateQueries({ queryKey: ['notes', 'list'] });
  },
  
  // Invalidate folders
  invalidateFolders: (queryClient: QueryClient) => {
    queryClient.invalidateQueries({ queryKey: ['notes', 'folders'] });
  },
  
  // Prefetch note for better UX
  prefetchNote: (queryClient: QueryClient, noteId: string, notesService: NotesService) => {
    queryClient.prefetchQuery({
      queryKey: ['notes', 'detail', noteId],
      queryFn: () => notesService.getNote(noteId),
      staleTime: cacheConfig.notes.detail.staleTime,
    });
  },
} as const;
