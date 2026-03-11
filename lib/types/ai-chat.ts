// AI Chat Types
// Based on backend models in src/models/ai/chat.rs

export interface ChatMessage {
  id: string;
  session_id: string;
  role: MessageRole;
  content: string;
  context_sources?: string[];
  created_at: string;
  updated_at: string;
}

export enum MessageRole {
  User = "user",
  Assistant = "assistant",
  System = "system",
}

export interface ChatSession {
  id: string;
  user_id: string;
  title: string;
  created_at: string;
  updated_at: string;
  last_message_at?: string;
}

export interface ChatRequest {
  message: string;
  session_id?: string;
  include_context?: boolean;
  max_context_vectors?: number;
  use_tools?: boolean;
}

export interface StreamingChatRequest {
  message: string;
  session_id?: string;
  include_context?: boolean;
  max_context_vectors?: number;
  use_tools?: boolean;
}

export interface ChatResponse {
  message: ChatMessage;
  session: ChatSession;
  context_sources: ContextSource[];
}

export interface ContextSource {
  vector_id: string;
  entity_id: string;
  data_type: string;
  snippet: string;
  similarity_score: number;
}

export interface ChatSessionDetailsResponse {
  session: ChatSession;
  messages: ChatMessage[];
  total_messages: number;
}

export interface ChatSessionListResponse {
  sessions: ChatSessionSummary[];
  total_count: number;
}

export interface ChatSessionSummary {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  last_message_at?: string;
  message_count: number;
}

export interface SessionListQuery {
  limit?: number;
  offset?: number;
}

export interface UpdateSessionTitleRequest {
  title: string;
}

export interface CreateSessionRequest {
  title?: string;
}

// API Response wrapper
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string;
}

// Error types
export interface ChatError {
  message: string;
  code?: string;
  details?: Record<string, unknown>;
}

// Hook state types
export interface ChatState {
  sessions: ChatSessionSummary[];
  totalSessionsCount: number;
  currentSession: ChatSession | null;
  messages: ChatMessage[];
  isLoading: boolean;
  isStreaming: boolean;
  error: ChatError | null;
}

export interface ChatOptions {
  use_tools?: boolean;
  include_context?: boolean;
  max_context_vectors?: number;
}

export interface ChatActions {
  sendMessage: (message: string, sessionId?: string, options?: ChatOptions) => Promise<void>;
  sendStreamingMessage: (message: string, sessionId?: string, options?: ChatOptions) => Promise<void>;
  createSession: (title?: string) => Promise<string>;
  updateSessionTitle: (sessionId: string, title: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  loadSessions: () => Promise<void>;
  loadSession: (sessionId: string) => Promise<void>;
  clearError: () => void;
}

export type UseAIChatReturn = ChatState & ChatActions;
