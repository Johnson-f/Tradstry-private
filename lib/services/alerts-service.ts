import { apiClient } from "@/lib/services/api-client";
import { apiConfig } from "@/lib/config/api";
import type {
  AlertRule,
  CreateAlertRuleRequest,
  UpdateAlertRuleRequest,
  PriceEvaluationInput,
} from "@/lib/types/alerts";

export const alertsService = {
  async listRules(): Promise<AlertRule[]> {
    return apiClient.get(apiConfig.endpoints.alerts.rules.base);
  },

  async createRule(payload: CreateAlertRuleRequest): Promise<AlertRule> {
    return apiClient.post(apiConfig.endpoints.alerts.rules.base, payload);
  },

  async updateRule(id: string, payload: UpdateAlertRuleRequest): Promise<AlertRule> {
    return apiClient.put(apiConfig.endpoints.alerts.rules.byId(id), payload);
  },

  async deleteRule(id: string): Promise<void> {
    await apiClient.delete(apiConfig.endpoints.alerts.rules.byId(id));
  },

  async evaluatePrices(prices: PriceEvaluationInput[]): Promise<{ dispatched: number }> {
    return apiClient.post(apiConfig.endpoints.alerts.base + "/evaluate/price", prices);
  },

  async sendTest(): Promise<{ ok: boolean }> {
    return apiClient.post(apiConfig.endpoints.alerts.test);
  },
};

export default alertsService;

