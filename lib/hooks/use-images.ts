"use client";

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { imagesService } from '@/lib/services/images-service';
import type {
  ImageUpdate,
  ImageUploadParams,
  ImageSearchParams,
  ImagePaginationParams,
  UseImagesReturn,
  UseImageReturn,
  UseImagesByNoteReturn,
  UseImagesPaginatedReturn,
  UseImageSearchReturn,
  UseImageUrlReturn,
  UseImageUploadReturn,
  UseImageUpdateReturn,
  UseImageDeleteReturn,
  UseImagesBulkDeleteReturn,
  UseImageReplaceReturn,
} from '@/lib/types/images';

// ==================== QUERY HOOKS ====================

/**
 * Hook to get all images for the current user
 */
export function useImages(): UseImagesReturn {
  const {
    data: images,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['images'],
    queryFn: () => imagesService.getImages(),
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
  });

  return {
    images: images ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to get a specific image by ID
 */
export function useImage(imageId: string): UseImageReturn {
  const {
    data: image,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['image', imageId],
    queryFn: () => imagesService.getImage(imageId),
    enabled: !!imageId,
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
  });

  return {
    image: image ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to get all images for a specific note
 */
export function useImagesByNote(noteId: string): UseImagesByNoteReturn {
  const {
    data: images,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['images', 'note', noteId],
    queryFn: () => imagesService.getImagesByNote(noteId),
    enabled: !!noteId,
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
  });

  return {
    images: images ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to get images with pagination
 */
export function useImagesPaginated(params?: ImagePaginationParams): UseImagesPaginatedReturn {
  const {
    data: images,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['images', 'paginated', params],
    queryFn: () => imagesService.getImagesPaginated(params),
    staleTime: 5 * 60 * 1000, // 5 minutes
    gcTime: 10 * 60 * 1000, // 10 minutes
  });

  return {
    images: images ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to search images
 */
export function useImageSearch(params: ImageSearchParams): UseImageSearchReturn {
  const {
    data: images,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['images', 'search', params.search_term],
    queryFn: () => imagesService.searchImages(params),
    enabled: !!params.search_term,
    staleTime: 2 * 60 * 1000, // 2 minutes (shorter for search results)
    gcTime: 5 * 60 * 1000, // 5 minutes
  });

  return {
    images: images ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

/**
 * Hook to get a signed URL for an image
 */
export function useImageUrl(imageId: string, expiresIn?: number): UseImageUrlReturn {
  const {
    data: urlData,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['image', 'url', imageId, expiresIn],
    queryFn: () => imagesService.getImageUrl(imageId, expiresIn),
    enabled: !!imageId,
    staleTime: (expiresIn ?? 3600) * 1000 * 0.8, // 80% of expiration time
    gcTime: (expiresIn ?? 3600) * 1000, // Full expiration time
  });

  return {
    url: urlData?.url ?? null,
    isLoading,
    error: error as Error | null,
    refetch,
  };
}

// ==================== MUTATION HOOKS ====================

/**
 * Hook to upload an image
 */
export function useImageUpload(): UseImageUploadReturn {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: (params: ImageUploadParams) => imagesService.uploadImage(params),
    onSuccess: (data) => {
      // Invalidate and refetch images queries
      queryClient.invalidateQueries({ queryKey: ['images'] });
      
      // If the image is associated with a note, invalidate that specific query
      if (data.note_id) {
        queryClient.invalidateQueries({ queryKey: ['images', 'note', data.note_id] });
      }
    },
  });

  return {
    uploadImage: mutation.mutateAsync,
    isUploading: mutation.isPending,
    error: mutation.error as Error | null,
  };
}

/**
 * Hook to update an image
 */
export function useImageUpdate(): UseImageUpdateReturn {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({ imageId, data }: { imageId: string; data: ImageUpdate }) =>
      imagesService.updateImage(imageId, data),
    onSuccess: (data, variables) => {
      // Invalidate and refetch related queries
      queryClient.invalidateQueries({ queryKey: ['images'] });
      queryClient.invalidateQueries({ queryKey: ['image', variables.imageId] });
      
      if (data.note_id) {
        queryClient.invalidateQueries({ queryKey: ['images', 'note', data.note_id] });
      }
    },
  });

  return {
    updateImage: (imageId: string, data: ImageUpdate) =>
      mutation.mutateAsync({ imageId, data }),
    isUpdating: mutation.isPending,
    error: mutation.error as Error | null,
  };
}

/**
 * Hook to delete an image
 */
export function useImageDelete(): UseImageDeleteReturn {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({ imageId, deleteFromStorage }: { imageId: string; deleteFromStorage?: boolean }) =>
      imagesService.deleteImage(imageId, deleteFromStorage),
    onSuccess: (data, variables) => {
      // Invalidate and refetch related queries
      queryClient.invalidateQueries({ queryKey: ['images'] });
      queryClient.invalidateQueries({ queryKey: ['image', variables.imageId] });
      
      if (data.deleted_record?.note_id) {
        queryClient.invalidateQueries({ 
          queryKey: ['images', 'note', data.deleted_record.note_id] 
        });
      }
    },
  });

  return {
    deleteImage: (imageId: string, deleteFromStorage?: boolean) =>
      mutation.mutateAsync({ imageId, deleteFromStorage }),
    isDeleting: mutation.isPending,
    error: mutation.error as Error | null,
  };
}

/**
 * Hook to delete all images for a note
 */
export function useImagesBulkDelete(): UseImagesBulkDeleteReturn {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({ noteId, deleteFromStorage }: { noteId: string; deleteFromStorage?: boolean }) =>
      imagesService.deleteImagesByNote(noteId, deleteFromStorage),
    onSuccess: (data, variables) => {
      // Invalidate and refetch related queries
      queryClient.invalidateQueries({ queryKey: ['images'] });
      queryClient.invalidateQueries({ queryKey: ['images', 'note', variables.noteId] });
      
      // Invalidate individual image queries for deleted images
      data.deleted_records.forEach((image) => {
        queryClient.invalidateQueries({ queryKey: ['image', image.id] });
      });
    },
  });

  return {
    deleteImagesByNote: (noteId: string, deleteFromStorage?: boolean) =>
      mutation.mutateAsync({ noteId, deleteFromStorage }),
    isDeleting: mutation.isPending,
    error: mutation.error as Error | null,
  };
}

/**
 * Hook to replace an image file
 */
export function useImageReplace(): UseImageReplaceReturn {
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: ({ imageId, file }: { imageId: string; file: File }) =>
      imagesService.replaceImage(imageId, file),
    onSuccess: (data, variables) => {
      // Invalidate and refetch related queries
      queryClient.invalidateQueries({ queryKey: ['images'] });
      queryClient.invalidateQueries({ queryKey: ['image', variables.imageId] });
      queryClient.invalidateQueries({ queryKey: ['image', 'url', variables.imageId] });
      
      if (data.note_id) {
        queryClient.invalidateQueries({ queryKey: ['images', 'note', data.note_id] });
      }
    },
  });

  return {
    replaceImage: (imageId: string, file: File) =>
      mutation.mutateAsync({ imageId, file }),
    isReplacing: mutation.isPending,
    error: mutation.error as Error | null,
  };
}

// ==================== UTILITY HOOKS ====================

/**
 * Hook to prefetch images for a note
 * Useful for preloading images when hovering over a note
 */
export function usePrefetchImagesByNote() {
  const queryClient = useQueryClient();

  return (noteId: string) => {
    queryClient.prefetchQuery({
      queryKey: ['images', 'note', noteId],
      queryFn: () => imagesService.getImagesByNote(noteId),
      staleTime: 5 * 60 * 1000,
    });
  };
}

/**
 * Hook to prefetch an image URL
 * Useful for preloading image URLs before displaying them
 */
export function usePrefetchImageUrl() {
  const queryClient = useQueryClient();

  return (imageId: string, expiresIn?: number) => {
    queryClient.prefetchQuery({
      queryKey: ['image', 'url', imageId, expiresIn],
      queryFn: () => imagesService.getImageUrl(imageId, expiresIn),
      staleTime: (expiresIn ?? 3600) * 1000 * 0.8,
    });
  };
}
