/**
 * Account Service - Frontend API integration for account management
 */

import apiClient from './api-client';
import { apiConfig } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';

export interface DeleteAccountResponse {
  success: boolean;
  message?: string;
  error?: string;
}
export interface StorageUsageData {
  used_bytes: number;
  limit_bytes: number;
  used_mb: number;
  limit_mb: number;
  remaining_bytes: number;
  remaining_mb: number;
  percentage_used: number;
}
export interface StorageUsageResponse {
  success: boolean;
  data?: StorageUsageData;
  error?: string;
}

class AccountService {
  /**
   * Get storage usage for the current user
   * @returns Promise resolving with storage usage data
   */
  async getStorageUsage(): Promise<StorageUsageData> {
    try {
      const response = await apiClient.get<StorageUsageResponse>(

        apiConfig.endpoints.user.storage
      );

      // apiClient.get returns the response data directly
      // Backend returns: { success: true, data: { ... } }
      if (!response.success || !response.data) {
        throw new Error(response.error || 'Failed to get storage usage');
      }

      return response.data;
    } catch (error) {
      console.error('Storage usage error:', error);
      throw error;
    } 
  }

  /**
   * Delete user account (irreversible)
   * Deletes all user data including database, storage, vectors, and auth account
   * @returns Promise resolving when account is deleted
   */
  async deleteAccount(): Promise<void> {
    try {
      const response = await apiClient.delete<DeleteAccountResponse>(
        apiConfig.endpoints.user.deleteAccount
      );

      if (!response.success) {
        throw new Error(response.error || 'Failed to delete account');
      }

      // Sign out after successful deletion
      const supabase = createClient();
      await supabase.auth.signOut();
    } catch (error) {
      console.error('Account deletion error:', error);
      throw error;
    }
  }
}

export const accountService = new AccountService();
export default accountService;