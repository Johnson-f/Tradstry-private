import { apiClient } from '@/lib/services/api-client';
import { apiConfig } from '@/lib/config/api';
import type {
  TradeTag,
  CreateTagRequest,
  UpdateTagRequest,
  TagQuery,
  ApiListResponse,
  ApiItemResponse,
} from '@/lib/types/trade-tags';

class TradeTagService {
  async getCategories(): Promise<ApiListResponse<string>> {
    return apiClient.get(apiConfig.endpoints.tradeTags.categories);
  }

  async listTags(query?: TagQuery): Promise<ApiListResponse<TradeTag>> {
    return apiClient.get(apiConfig.endpoints.tradeTags.base, { params: query });
  }

  async getTag(id: string): Promise<ApiItemResponse<TradeTag>> {
    return apiClient.get(apiConfig.endpoints.tradeTags.byId(id));
  }

  async createTag(payload: CreateTagRequest): Promise<ApiItemResponse<TradeTag>> {
    return apiClient.post(apiConfig.endpoints.tradeTags.base, payload);
  }

  async updateTag(id: string, payload: UpdateTagRequest): Promise<ApiItemResponse<TradeTag>> {
    return apiClient.put(apiConfig.endpoints.tradeTags.byId(id), payload);
  }

  async deleteTag(id: string): Promise<{ success: boolean; message: string }> {
    return apiClient.delete(apiConfig.endpoints.tradeTags.byId(id));
  }

  // Trade associations
  async getStockTradeTags(tradeId: number): Promise<ApiListResponse<TradeTag>> {
    return apiClient.get(apiConfig.endpoints.tradeTags.stockTrade(tradeId));
  }

  async addTagsToStockTrade(tradeId: number, tagIds: string[]): Promise<{ success: boolean; message: string; added: number; skipped: number; }> {
    return apiClient.post(apiConfig.endpoints.tradeTags.stockTrade(tradeId), { tag_ids: tagIds });
  }

  async removeTagFromStockTrade(tradeId: number, tagId: string): Promise<{ success: boolean; message: string }> {
    return apiClient.delete(`${apiConfig.endpoints.tradeTags.stockTrade(tradeId)}/${tagId}`);
  }

  async getOptionTradeTags(tradeId: number): Promise<ApiListResponse<TradeTag>> {
    return apiClient.get(apiConfig.endpoints.tradeTags.optionTrade(tradeId));
  }

  async addTagsToOptionTrade(tradeId: number, tagIds: string[]): Promise<{ success: boolean; message: string; added: number; skipped: number; }> {
    return apiClient.post(apiConfig.endpoints.tradeTags.optionTrade(tradeId), { tag_ids: tagIds });
  }

  async removeTagFromOptionTrade(tradeId: number, tagId: string): Promise<{ success: boolean; message: string }> {
    return apiClient.delete(`${apiConfig.endpoints.tradeTags.optionTrade(tradeId)}/${tagId}`);
  }
}

export const tradeTagService = new TradeTagService();