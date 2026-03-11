// AI Chat Service - Fixed streaming implementation
// Frontend service for communicating with backend AI chat endpoints

import { apiConfig, getFullUrl } from '@/lib/config/api';
import { createClient } from '@/lib/supabase/client';
import type {
  ChatRequest,
  StreamingChatRequest,
  ChatResponse,
  ChatSession,
  ChatSessionDetailsResponse,
  ChatSessionListResponse,
  CreateSessionRequest,
  SessionListQuery,
  ApiResponse,
  ChatError,
} from '@/lib/types/ai-chat';

export class AIChatService {
  private supabase;

  constructor() {
    this.supabase = createClient();
  }

  // Lazy getter for base URL to avoid SSR issues
  private getBaseUrl(): string {
    if (typeof window === 'undefined') {
      // SSR fallback
      return '';
    }
    try {
      return getFullUrl(
        apiConfig.endpoints.ai.chat.base
      );
    } catch (error) {
      console.error('Error getting base URL:', error);
      return '';
    }
  }

  // Helper to safely get endpoint
  private getEndpoint(path: string, ...args: unknown[]): string {
    if (typeof window === 'undefined') {
      return '';
    }
    try {
      const chatEndpoints = apiConfig.endpoints.ai.chat;
      if (!chatEndpoints) {
        return '';
      }
      
      // Handle nested paths like 'sessions.base', 'sessions.byId'
      const pathParts = path.split('.');
      let endpoint: unknown = chatEndpoints;
      
      for (const part of pathParts) {
        if (endpoint && typeof endpoint === 'object' && endpoint !== null) {
          const endpointObj = endpoint as Record<string, unknown>;
          if (part in endpointObj) {
            endpoint = endpointObj[part];
          } else {
            // Fallback to base if path not found
            endpoint = chatEndpoints.base;
            break;
          }
        } else {
          // Fallback to base if path not found
          endpoint = chatEndpoints.base;
          break;
        }
      }
      
      // If endpoint is a function, call it with args
      if (typeof endpoint === 'function') {
        endpoint = endpoint(...args) as string;
      }
      
      return getFullUrl((endpoint as string) || chatEndpoints.base);
    } catch (error) {
      console.error('Error getting endpoint:', error);
      return '';
    }
  }

  /**
   * Send a chat message and get response
   */
  async sendMessage(request: ChatRequest): Promise<ChatResponse> {
    try {
      const token = await this.getAuthToken();
      if (!token) {
        throw new Error('User not authenticated - please log in');
      }
      
      const response = await fetch(this.getEndpoint('send'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result: ApiResponse<ChatResponse> = await response.json();
      
      if (!result.success || !result.data) {
        throw new Error(result.message || 'Failed to send message');
      }

      return result.data;
    } catch (error) {
      console.error('Error sending chat message:', error);
      throw this.handleError(error);
    }
  }

  /**
   * Send a streaming chat message with 401 retry logic
   */
  async sendStreamingMessage(
    request: StreamingChatRequest,
    onChunk: (chunk: string) => void,
    onComplete: () => void,
    onError: (error: Error) => void
  ): Promise<void> {
    const makeRequest = async (isRetry: boolean = false): Promise<void> => {
      try {
        const token = await this.getAuthToken();
        if (!token) {
          throw new Error('User not authenticated - please log in');
        }
        
        const streamUrl = this.getEndpoint('stream');
        console.log('Making streaming request to:', streamUrl);
        console.log('Request body:', JSON.stringify(request));
        console.log('Authorization header:', `Bearer ${token.substring(0, 20)}...`);
        
        const response = await fetch(streamUrl, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${token}`,
          },
          body: JSON.stringify(request),
        });

        console.log('Response status:', response.status);
        console.log('Response headers:', Object.fromEntries(response.headers.entries()));
        
        if (!response.ok) {
          if (response.status === 401 && !isRetry) {
            // Try refreshing token and retry once
            console.log('Received 401, attempting token refresh and retry...');
            await this.supabase.auth.refreshSession();
            return makeRequest(true);
          }
          throw new Error(`HTTP error! status: ${response.status}`);
        }

        console.log('Response body available:', !!response.body);
        
        const reader = response.body?.getReader();
        if (!reader) {
          console.error('No response body reader available');
          throw new Error('No response body reader available');
        }

        console.log('Starting to read stream...');
        const decoder = new TextDecoder();
        let buffer = '';
        let totalChunksReceived = 0;
        let totalTokensProcessed = 0;

        try {
          while (true) {
            const { done, value } = await reader.read();
            
            if (done) {
              console.log('Stream reading done');
              
              // Process any remaining data in buffer
              if (buffer.trim()) {
                console.log('Processing remaining buffer:', buffer);
                const trimmed = buffer.trim();
                try {
                  const data = JSON.parse(trimmed);
                  console.log('Parsed final data:', data);
                  if (data.chunk) {
                    console.log('Emitting final chunk:', data.chunk);
                    onChunk(data.chunk);
                    totalTokensProcessed++;
                  }
                } catch (e) {
                  console.warn('Failed to parse final buffer:', trimmed, e);
                }
              }
              
              console.log('Stream completed successfully');
              console.log('Total chunks received:', totalChunksReceived);
              console.log('Total tokens processed:', totalTokensProcessed);
              onComplete();
              break;
            }

            totalChunksReceived++;
            
            // Decode the chunk
            const textChunk = decoder.decode(value, { stream: true });
            console.log(`Chunk #${totalChunksReceived} raw text (${textChunk.length} bytes):`, textChunk);
            
            // Add to buffer
            buffer += textChunk;
            
            // Split by newlines - each line should be a complete JSON object
            const lines = buffer.split('\n');
            
            // The last element might be incomplete, so keep it in the buffer
            // If buffer ends with \n, the last element will be empty string, which is fine
            buffer = lines.pop() || '';
            
            console.log(`Processing ${lines.length} complete lines, buffer size: ${buffer.length}`);

            // Process each complete line
            for (const line of lines) {
              const trimmed = line.trim();
              
              if (!trimmed) {
                console.log('Skipping empty line');
                continue;
              }
              
              console.log('Processing line:', trimmed);
              
              try {
                const data = JSON.parse(trimmed);
                console.log('Parsed data:', data);
                
                if (data.chunk !== undefined) {
                  console.log('Emitting chunk:', data.chunk);
                  onChunk(data.chunk);
                  totalTokensProcessed++;
                } else if (data.error) {
                  console.error('Stream error:', data.error);
                  onError(new Error(data.error));
                  return;
                } else {
                  console.warn('Unknown data format:', data);
                }
              } catch (e) {
                console.error('Failed to parse line as JSON:', trimmed);
                console.error('Parse error:', e);
                // Continue processing other lines
              }
            }
          }
        } catch (streamError) {
          console.error('Error while reading stream:', streamError);
          throw streamError;
        }
      } catch (error) {
        console.error('Error in sendStreamingMessage:', error);
        onError(this.handleError(error) as Error);
      }
    };

    await makeRequest();
  }

  /**
   * Get user's chat sessions
   */
  async getSessions(query: SessionListQuery = {}): Promise<ChatSessionListResponse> {
    try {
      const params = new URLSearchParams();
      if (query.limit) params.append('limit', query.limit.toString());
      if (query.offset) params.append('offset', query.offset.toString());

      const url = `${this.getEndpoint('sessions.base')}?${params}`;
      
      const token = await this.getAuthToken();
      if (!token) {
        throw new Error('User not authenticated - please log in');
      }
      
      const response = await fetch(url, {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result: ApiResponse<ChatSessionListResponse> = await response.json();
      
      if (!result.success || !result.data) {
        throw new Error(result.message || 'Failed to get sessions');
      }

      return result.data;
    } catch (error) {
      console.error('Error getting chat sessions:', error);
      throw this.handleError(error);
    }
  }

  /**
   * Get specific chat session with messages
   */
  async getSession(sessionId: string): Promise<ChatSessionDetailsResponse> {
    try {
      const token = await this.getAuthToken();
      if (!token) {
        throw new Error('User not authenticated - please log in');
      }
      
      const response = await fetch(
        this.getEndpoint('sessions.byId', sessionId),
        {
          method: 'GET',
          headers: {
            'Authorization': `Bearer ${token}`,
          },
        }
      );

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result: ApiResponse<ChatSessionDetailsResponse> = await response.json();
      
      if (!result.success || !result.data) {
        throw new Error(result.message || 'Failed to get session');
      }

      return result.data;
    } catch (error) {
      console.error('Error getting chat session:', error);
      throw this.handleError(error);
    }
  }

  /**
   * Create a new chat session
   */
  async createSession(request: CreateSessionRequest = {}): Promise<ChatSession> {
    try {
      const token = await this.getAuthToken();
      if (!token) {
        throw new Error('User not authenticated - please log in');
      }
      
      const response = await fetch(this.getEndpoint('sessions.base'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result: ApiResponse<ChatSession> = await response.json();
      
      if (!result.success || !result.data) {
        throw new Error(result.message || 'Failed to create session');
      }

      return result.data;
    } catch (error) {
      console.error('Error creating chat session:', error);
      throw this.handleError(error);
    }
  }

  /**
   * Update chat session title
   */
  async updateSessionTitle(sessionId: string, title: string): Promise<void> {
    try {
      const token = await this.getAuthToken();
      if (!token) {
        throw new Error('User not authenticated - please log in');
      }
      
      const response = await fetch(
        this.getEndpoint('sessions.updateTitle', sessionId),
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${token}`,
          },
          body: JSON.stringify({ title }),
        }
      );

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result: ApiResponse<void> = await response.json();
      
      if (!result.success) {
        throw new Error(result.message || 'Failed to update session title');
      }
    } catch (error) {
      console.error('Error updating session title:', error);
      throw this.handleError(error);
    }
  }

  /**
   * Delete a chat session
   */
  async deleteSession(sessionId: string): Promise<void> {
    try {
      const token = await this.getAuthToken();
      if (!token) {
        throw new Error('User not authenticated - please log in');
      }
      
      const response = await fetch(
        this.getEndpoint('sessions.byId', sessionId),
        {
          method: 'DELETE',
          headers: {
            'Authorization': `Bearer ${token}`,
          },
        }
      );

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const result: ApiResponse<void> = await response.json();
      
      if (!result.success) {
        throw new Error(result.message || 'Failed to delete session');
      }
    } catch (error) {
      console.error('Error deleting chat session:', error);
      throw this.handleError(error);
    }
  }

  /**
   * Get authentication token from Supabase session with refresh capability
   */
  private async getAuthToken(): Promise<string | null> {
    try {
      const { data: { session }, error } = await this.supabase.auth.getSession();
      
      if (error) {
        console.error('Error getting session:', error);
        return null;
      }
      
      if (!session?.access_token) {
        console.log('No authentication token found - user not logged in');
        return null;
      }
      
      // Check if token expires soon (within 5 minutes)
      const tokenExpiry = session.expires_at ? new Date(session.expires_at * 1000) : null;
      const now = new Date();
      const fiveMinutesFromNow = new Date(now.getTime() + 5 * 60 * 1000);
      
      if (tokenExpiry && tokenExpiry < fiveMinutesFromNow) {
        console.log('Token expires soon, refreshing...');
        const { data: { session: refreshedSession }, error: refreshError } = await this.supabase.auth.refreshSession();
        
        if (refreshError) {
          console.error('Error refreshing session:', refreshError);
          return null;
        }
        
        if (refreshedSession?.access_token) {
          console.log('Token refreshed successfully');
          return refreshedSession.access_token;
        }
      }
      
      return session.access_token;
    } catch (error) {
      console.error('Error getting auth token:', error);
      return null;
    }
  }

  /**
   * Handle and format errors
   */
  private handleError(error: unknown): ChatError {
    if (error instanceof Error) {
      return {
        message: error.message,
        code: 'CHAT_ERROR',
      };
    }
    
    return {
      message: 'An unknown error occurred',
      code: 'UNKNOWN_ERROR',
    };
  }
}

// Export singleton instance
export const aiChatService = new AIChatService();