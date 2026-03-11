import { apiClient, type ApiError } from './api-client';
import { apiConfig } from '@/lib/config/api';
import type {
  CreateTradeNoteRequest,
  UpdateTradeNoteRequest,
  CreateTradeNoteForTradeRequest,
  TradeNoteResponse,
  TradeNoteListResponse,
} from '@/lib/types/trade-note';

class TradeNoteService {
  // Generic trade notes (not linked to specific trades)
  async createNote(payload: CreateTradeNoteRequest): Promise<TradeNoteResponse> {
    return apiClient.post(apiConfig.endpoints.tradeNotes.base, payload);
  }

  async getNote(noteId: string): Promise<TradeNoteResponse> {
    return apiClient.get(apiConfig.endpoints.tradeNotes.byId(noteId));
  }

  async listNotes(): Promise<TradeNoteListResponse> {
    return apiClient.get(apiConfig.endpoints.tradeNotes.base);
  }

  async updateNote(noteId: string, payload: UpdateTradeNoteRequest): Promise<TradeNoteResponse> {
    return apiClient.put(apiConfig.endpoints.tradeNotes.byId(noteId), payload);
  }

  async deleteNote(noteId: string): Promise<{ success: boolean; message: string }> {
    return apiClient.delete(apiConfig.endpoints.tradeNotes.byId(noteId));
  }

  // Trade-linked notes
  async upsertTradeNote(
    tradeType: 'stock' | 'option',
    tradeId: number,
    payload: CreateTradeNoteForTradeRequest
  ): Promise<TradeNoteResponse> {
    return apiClient.post(
      apiConfig.endpoints.tradeNotes.byTrade(tradeType, tradeId),
      payload
    );
  }

  async getTradeNote(tradeType: 'stock' | 'option', tradeId: number): Promise<TradeNoteResponse> {
    try {
      return await apiClient.get(apiConfig.endpoints.tradeNotes.byTrade(tradeType, tradeId));
    } catch (error: unknown) {
      // Handle 404 as not found (note doesn't exist yet)
      if (this.isApiError(error) && error.status === 404) {
        return { success: false, message: 'Trade note not found', data: undefined };
      }
      throw error;
    }
  }

  // Type guard for ApiError
  private isApiError(error: unknown): error is ApiError {
    return (
      typeof error === 'object' &&
      error !== null &&
      'status' in error &&
      typeof (error as ApiError).status === 'number'
    );
  }

  async deleteTradeNote(
    tradeType: 'stock' | 'option',
    tradeId: number
  ): Promise<{ success: boolean; message: string }> {
    return apiClient.delete(apiConfig.endpoints.tradeNotes.byTrade(tradeType, tradeId));
  }
}

export const tradeNoteService = new TradeNoteService();