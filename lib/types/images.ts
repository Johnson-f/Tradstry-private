/**
 * Images Types - TypeScript definitions for Images API
 * Matches backend models defined in backend/models/images.py
 */

// Base image interface
export interface ImageBase {
  note_id?: string | null;
  filename: string;
  original_filename: string;
  file_path: string;
  file_size: number;
  mime_type: string;
  width?: number | null;
  height?: number | null;
  alt_text?: string | null;
  caption?: string | null;
}

// Image creation request
export type ImageCreate = ImageBase;

// Image update request (all fields optional)
export interface ImageUpdate {
  note_id?: string | null;
  filename?: string;
  original_filename?: string;
  file_path?: string;
  file_size?: number;
  mime_type?: string;
  width?: number | null;
  height?: number | null;
  alt_text?: string | null;
  caption?: string | null;
}

// Image database representation
export interface Image extends ImageBase {
  id: string;
  user_id: string;
  created_at: string;
  updated_at: string;
}

// Image upsert response
export type ImageUpsertResponse = Image;

// Image delete response
export interface ImageDeleteResponse {
  success: boolean;
  deleted_record?: Image | null;
  error?: string | null;
}

// Bulk image delete response
export interface BulkImageDeleteResponse {
  success: boolean;
  deleted_count: number;
  deleted_records: Image[];
  error?: string | null;
}

// Image search parameters
export interface ImageSearchParams {
  search_term: string;
}

// Image pagination parameters
export interface ImagePaginationParams {
  limit?: number;
  offset?: number;
}

// Image upload parameters
export interface ImageUploadParams {
  file: File;
  note_id?: string;
  alt_text?: string;
  caption?: string;
}

// Image URL response
export interface ImageUrlResponse {
  url: string;
  expires_in: number;
}

// Generic API response types
export interface ApiSuccessResponse {
  success: boolean;
  message?: string;
}

// Hook return types
export interface UseImagesReturn {
  images: Image[] | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseImageReturn {
  image: Image | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseImagesByNoteReturn {
  images: Image[] | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseImagesPaginatedReturn {
  images: Image[] | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseImageSearchReturn {
  images: Image[] | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseImageUrlReturn {
  url: string | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

// Mutation return types
export interface UseImageUploadReturn {
  uploadImage: (params: ImageUploadParams) => Promise<ImageUpsertResponse>;
  isUploading: boolean;
  error: Error | null;
}

export interface UseImageUpdateReturn {
  updateImage: (imageId: string, data: ImageUpdate) => Promise<ImageUpsertResponse>;
  isUpdating: boolean;
  error: Error | null;
}

export interface UseImageDeleteReturn {
  deleteImage: (imageId: string, deleteFromStorage?: boolean) => Promise<ImageDeleteResponse>;
  isDeleting: boolean;
  error: Error | null;
}

export interface UseImagesBulkDeleteReturn {
  deleteImagesByNote: (noteId: string, deleteFromStorage?: boolean) => Promise<BulkImageDeleteResponse>;
  isDeleting: boolean;
  error: Error | null;
}

export interface UseImageReplaceReturn {
  replaceImage: (imageId: string, file: File) => Promise<ImageUpsertResponse>;
  isReplacing: boolean;
  error: Error | null;
}
