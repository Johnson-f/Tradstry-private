import { apiClient } from './api-client';
import { apiConfig } from '@/lib/config/api';

// Track ongoing requests to prevent duplicates
const ongoingRequests = new Map<string, Promise<UserInitializationResponse>>();

export interface UserInitializationRequest {
  email: string;
  user_id: string;
}

// Updated to match backend response structure exactly
export interface UserInitializationResponse {
  success: boolean;
  message: string;  // REQUIRED field from backend
  database_url?: string;
  database_token?: string;
  schema_synced?: boolean;
  schema_version?: string;
  cache_preloaded?: boolean;
  cache_status?: string;
  user_id?: string;
}

/**
 * Initialize a new user after successful sign-up with retry logic
 * This creates necessary user records in the backend database
 * Implements exponential backoff and proper error handling
 */
export const initializeUser = async (
  email: string,
  userId: string,
  retryCount: number = 2,
  retryDelay: number = 1000
): Promise<UserInitializationResponse> => {
  // Check if there's already an ongoing request for this user
  const requestKey = `init-${userId}`;
  if (ongoingRequests.has(requestKey)) {
    return ongoingRequests.get(requestKey)!;
  }

  // Create the initialization promise
  const initPromise = performInitialization(email, userId, retryCount, retryDelay);
  
  // Store the promise to prevent duplicates
  ongoingRequests.set(requestKey, initPromise);
  
  try {
    const result = await initPromise;
    return result;
  } finally {
    // Clean up the ongoing request
    ongoingRequests.delete(requestKey);
  }
};

async function performInitialization(
  email: string,
  userId: string,
  retryCount: number,
  retryDelay: number
): Promise<UserInitializationResponse> {
  let lastError: unknown = null;
  
  for (let attempt = 1; attempt <= retryCount; attempt++) {
    try {
      const response = await apiClient.post<UserInitializationResponse>(
        apiConfig.endpoints.user.initialize,
        {
          email,
          user_id: userId,
        }
      );

      // Check if the response indicates success
      if (response.success) {
        return response;
      } else {
        // If the API returns success: false, treat it as an error
        throw new Error(response.message || 'Initialization failed');
      }
    } catch (error: unknown) {
      lastError = error;
      
      // Extract error message from various error types
      const getErrorMessage = (err: unknown): string => {
        if (err && typeof err === 'object') {
          if ('message' in err && typeof err.message === 'string') {
            return err.message;
          }
          if ('response' in err && err.response && typeof err.response === 'object') {
            const response = err.response as { data?: { message?: string } };
            if (response.data?.message) {
              return response.data.message;
            }
          }
        }
        return 'Failed to initialize user account';
      };
      
      // If this is the last attempt, return the error response
      if (attempt === retryCount) {
        const errorMessage = getErrorMessage(error);
        
        return {
          success: false,
          message: errorMessage,
        };
      }
      
      // Wait before retrying (exponential backoff)
      const delay = retryDelay * Math.pow(2, attempt - 1);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }

  // Fallback return (should never be reached)
  const getFallbackMessage = (err: unknown): string => {
    if (err instanceof Error) {
      return err.message;
    }
    return 'Unexpected error during user initialization';
  };
  
  return {
    success: false,
    message: getFallbackMessage(lastError),
  };
}

/**
 * Response from the check endpoint
 */
export interface CheckUserResponse {
  exists: boolean;
  database_url?: string;
  created_at?: string;
  schema_version?: string;
  needs_sync?: boolean;
  current_app_version?: string;
}

// Cache key for session storage
const CHECK_CACHE_KEY = 'user_db_check_';
const CHECK_CACHE_TTL = 5 * 60 * 1000; // 5 minutes

interface CachedCheckResult {
  response: CheckUserResponse;
  timestamp: number;
}

/**
 * Check if a user has been properly initialized
 * Uses session storage cache to avoid repeated API calls on page refresh
 * @param userId - The user ID to check
 * @param skipCache - Force a fresh check, bypassing cache
 * @returns CheckUserResponse with exists, needs_sync, and version info
 */
export const checkUserInitialization = async (
  userId: string,
  skipCache: boolean = false
): Promise<CheckUserResponse> => {
  const cacheKey = CHECK_CACHE_KEY + userId;
  
  // Try to get from cache first (unless skipCache is true)
  if (!skipCache && typeof window !== 'undefined') {
    try {
      const cached = sessionStorage.getItem(cacheKey);
      if (cached) {
        const parsed: CachedCheckResult = JSON.parse(cached);
        const age = Date.now() - parsed.timestamp;
        if (age < CHECK_CACHE_TTL) {
          return parsed.response;
        }
      }
    } catch {
      // Ignore cache errors
    }
  }
  
  try {
    const response = await apiClient.get<CheckUserResponse>(
      apiConfig.endpoints.user.check(userId)
    );
    
    // Cache the result
    if (typeof window !== 'undefined') {
      try {
        const cacheEntry: CachedCheckResult = {
          response,
          timestamp: Date.now(),
        };
        sessionStorage.setItem(cacheKey, JSON.stringify(cacheEntry));
      } catch {
        // Ignore cache errors
      }
    }
    
    return response;
  } catch (error: unknown) {
    console.error('Error checking user initialization:', error);
    return { exists: false };
  }
};

/**
 * Simple check if user database exists (backward compatible)
 */
export const isUserInitialized = async (userId: string): Promise<boolean> => {
  const result = await checkUserInitialization(userId);
  return result.exists;
};

/**
 * Clear the check cache for a user (call after initialization or sync)
 */
export const clearUserCheckCache = (userId: string): void => {
  if (typeof window !== 'undefined') {
    try {
      sessionStorage.removeItem(CHECK_CACHE_KEY + userId);
    } catch {
      // Ignore errors
    }
  }
};