import { apiClient } from "@/lib/services/api-client";
import { apiConfig } from "@/lib/config/api";

// AI action types
export type NotebookAIAction =
  | "correct_spelling"
  | "improve_writing"
  | "summarize"
  | "autocomplete"
  | "make_shorter"
  | "make_longer"
  | "simplify_language"
  | "fix_grammar";

// Request payload
export interface NotebookAIRequest {
  action: NotebookAIAction;
  content: string;
  context?: string;
  note_id?: string;
  request_id: string;
}

// Response from REST API (accepted)
export interface NotebookAIAcceptedResponse {
  success: boolean;
  message: string;
  data: null;
}

// WebSocket progress event
export interface NotebookAIProgressEvent {
  request_id: string;
  action: NotebookAIAction;
  status: "processing" | "complete" | "error";
}

// WebSocket chunk event
export interface NotebookAIChunkEvent {
  request_id: string;
  chunk: string;
  is_complete: boolean;
}

// WebSocket complete event
export interface NotebookAICompleteEvent {
  request_id: string;
  action: NotebookAIAction;
  result: string;
  original_content: string;
}

// Generate unique request ID
export function generateRequestId(): string {
  return `notebook-ai-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

class NotebookAIService {
  /**
   * Send an AI processing request
   * Results will be streamed via WebSocket
   */
  async process(request: NotebookAIRequest): Promise<NotebookAIAcceptedResponse> {
    return apiClient.post(apiConfig.endpoints.notebook.ai.process, request);
  }

  /**
   * Correct spelling in text
   */
  async correctSpelling(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "correct_spelling",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }

  /**
   * Fix grammar in text
   */
  async fixGrammar(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "fix_grammar",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }

  /**
   * Improve writing quality
   */
  async improveWriting(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "improve_writing",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }

  /**
   * Summarize text
   */
  async summarize(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "summarize",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }

  /**
   * Autocomplete text
   */
  async autocomplete(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "autocomplete",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }

  /**
   * Make text shorter
   */
  async makeShorter(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "make_shorter",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }

  /**
   * Make text longer
   */
  async makeLonger(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "make_longer",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }

  /**
   * Simplify language
   */
  async simplifyLanguage(
    content: string,
    options?: { context?: string; noteId?: string }
  ): Promise<string> {
    const requestId = generateRequestId();
    await this.process({
      action: "simplify_language",
      content,
      context: options?.context,
      note_id: options?.noteId,
      request_id: requestId,
    });
    return requestId;
  }
}

export const notebookAIService = new NotebookAIService();
