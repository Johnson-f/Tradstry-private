export type AlertType = "price" | "system";

export type AlertChannel = "push" | "in_app" | "email";

export interface AlertRule {
  id: string;
  user_id: string;
  alert_type: AlertType;
  symbol?: string | null;
  operator?: string | null;
  threshold?: number | null;
  channels: AlertChannel[];
  cooldown_seconds?: number | null;
  last_fired_at?: string | null;
  note?: string | null;
  max_triggers?: number | null;
  fired_count: number;
  is_active: boolean;
  metadata?: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
}

export interface CreateAlertRuleRequest {
  alert_type: AlertType;
  symbol?: string;
  operator?: string;
  threshold?: number;
  channels?: AlertChannel[];
  cooldown_seconds?: number;
  note?: string;
  max_triggers?: number;
  metadata?: Record<string, unknown>;
}

export interface UpdateAlertRuleRequest {
  symbol?: string;
  operator?: string;
  threshold?: number;
  channels?: AlertChannel[];
  cooldown_seconds?: number;
  last_fired_at?: string;
  note?: string;
  max_triggers?: number;
  fired_count?: number;
  is_active?: boolean;
  metadata?: Record<string, unknown>;
}

export interface PriceEvaluationInput {
  symbol: string;
  price: number;
  direction?: string;
  percent_change?: number;
  timestamp?: string;
}

