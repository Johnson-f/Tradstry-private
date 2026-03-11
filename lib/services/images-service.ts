/**
 * Images Service - Frontend API integration for Images endpoints
 * Connects to backend routes defined in backend/routers/images.py
 */

import apiClient from './api-client';
import { apiConfig } from '@/lib/config/api';
import {
  Image,
  ImageUpdate,
  ImageUpsertResponse,
  ImageDeleteResponse,
  BulkImageDeleteResponse,
  ImageSearchParams,
  ImagePaginationParams,
  ImageUploadParams,
  ImageUrlResponse,
} from '@/lib/types/images';

class ImagesService {
  // ==================== UPLOAD OPERATIONS ====================

  /**
   * Upload an image file to storage and create database record
   * @route POST /api/images/upload
   * @param params - Upload parameters including file and metadata
   * @returns Image upsert response with created image data
   */
  async uploadImage(params: ImageUploadParams): Promise<ImageUpsertResponse> {
    const formData = new FormData();
    formData.append('file', params.file);
    
    if (params.note_id) {
      formData.append('trade_note_id', params.note_id);
    }
    if (params.alt_text) {
      formData.append('alt_text', params.alt_text);
    }
    if (params.caption) {
      formData.append('caption', params.caption);
    }
    return apiClient.post<ImageUpsertResponse>(apiConfig.endpoints.images.upload, formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
    });
  }

  // ==================== RETRIEVE OPERATIONS ====================

  /**
   * Get all images for the current user
   * @route GET /api/images/
   * @returns List of all user's images
   */
  async getImages(): Promise<Image[]> {
    return apiClient.get<Image[]>(apiConfig.endpoints.images.base);
  }

  /**
   * Get a specific image by ID
   * @route GET /api/images/{image_id}
   * @param imageId - The ID of the image to retrieve
   * @returns The image with the specified ID
   */
  async getImage(imageId: string): Promise<Image> {
    return apiClient.get<Image>(apiConfig.endpoints.images.byId(imageId));
  }

  /**
   * Get all images for a specific note
   * @route GET /api/images/note/{note_id}
   * @param noteId - The ID of the note
   * @returns List of images associated with the note
   */
  async getImagesByNote(noteId: string): Promise<Image[]> {
    return apiClient.get<Image[]>(apiConfig.endpoints.images.byTradeNote(noteId));
  }

  /**
   * Get images with pagination
   * @route GET /api/images/paginated/list
   * @param params - Pagination parameters
   * @returns Paginated list of images
   */
  async getImagesPaginated(params?: ImagePaginationParams): Promise<Image[]> {
    return apiClient.get<Image[]>('/images/paginated/list', { params });
  }

  /**
   * Search images by filename, alt text, or caption
   * @route GET /api/images/search/query
   * @param params - Search parameters
   * @returns List of images matching the search term
   */
  async searchImages(params: ImageSearchParams): Promise<Image[]> {
    return apiClient.get<Image[]>('/images/search/query', { params });
  }

  // ==================== UPDATE OPERATIONS ====================

  /**
   * Update image metadata (not the file itself)
   * @route PUT /api/images/{image_id}
   * @param imageId - The ID of the image to update
   * @param data - The partial image data to update
   * @returns Updated image data
   */
  async updateImage(imageId: string, data: ImageUpdate): Promise<ImageUpsertResponse> {
    return apiClient.put<ImageUpsertResponse>(apiConfig.endpoints.images.byId(imageId), data);
  }

  // ==================== DELETE OPERATIONS ====================

  /**
   * Delete an image
   * @route DELETE /api/images/{image_id}
   * @param imageId - The ID of the image to delete
   * @param deleteFromStorage - Whether to also delete from storage (default: true)
   * @returns Delete response with success status and deleted record
   */
  async deleteImage(imageId: string, deleteFromStorage: boolean = true): Promise<ImageDeleteResponse> {
    return apiClient.delete<ImageDeleteResponse>(apiConfig.endpoints.images.byId(imageId), {
      params: { delete_from_storage: deleteFromStorage },
    });
  }

  /**
   * Delete all images for a specific note
   * @route DELETE /api/images/note/{note_id}/all
   * @param noteId - The ID of the note
   * @param deleteFromStorage - Whether to also delete from storage (default: true)
   * @returns Bulk delete response with count and deleted records
   */
  async deleteImagesByNote(noteId: string, deleteFromStorage: boolean = true): Promise<BulkImageDeleteResponse> {
    return apiClient.delete<BulkImageDeleteResponse>(`${apiConfig.endpoints.images.byTradeNote(noteId)}/all`, {
      params: { delete_from_storage: deleteFromStorage },
    });
  }

  // ==================== STORAGE OPERATIONS ====================

  /**
   * Get a signed URL for accessing an image
   * @route GET /api/images/{image_id}/url
   * @param imageId - The ID of the image
   * @param expiresIn - URL expiration time in seconds (default: 3600)
   * @returns Signed URL and expiration info
   */
  async getImageUrl(imageId: string, expiresIn: number = 3600): Promise<ImageUrlResponse> {
    return apiClient.get<ImageUrlResponse>(apiConfig.endpoints.images.byId(imageId) + '/url', {
      params: { expires_in: expiresIn },
    });
  }

  /**
   * Replace an existing image file while keeping the same database record
   * @route POST /api/images/{image_id}/replace
   * @param imageId - The ID of the image to replace
   * @param file - The new image file
   * @returns Updated image data
   */
  async replaceImage(imageId: string, file: File): Promise<ImageUpsertResponse> {
    const formData = new FormData();
    formData.append('file', file);

    return apiClient.post<ImageUpsertResponse>(`${apiConfig.endpoints.images.byId(imageId)}/replace`, formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
    });
  }
}

// Export singleton instance
export const imagesService = new ImagesService();
export default imagesService;