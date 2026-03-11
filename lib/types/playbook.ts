export type RuleType = 'entry_criteria' | 'exit_criteria' | 'market_factor';
export type TradeTypeForTagging = 'stock' | 'option';

export interface Playbook {
  id: string;
  name: string;
  description?: string | null;
  icon?: string | null;
  emoji?: string | null;
  color?: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreatePlaybookRequest {
  name: string;
  description?: string | null;
  icon?: string | null;
  emoji?: string | null;
  color?: string | null;
}

export interface UpdatePlaybookRequest {
  name?: string;
  description?: string | null;
  icon?: string | null;
  emoji?: string | null;
  color?: string | null;
}

export interface PlaybookQuery {
  name?: string;
  search?: string;
  limit?: number;
  offset?: number;
  page?: number;
  page_size?: number;
}

export interface PlaybookRule {
  id: string;
  playbook_id: string;
  rule_type: RuleType;
  title: string;
  description?: string | null;
  order_position: number;
  created_at: string;
  updated_at: string;
}

export interface CreateRuleRequest {
  rule_type: RuleType;
  title: string;
  description?: string | null;
  order_position?: number;
}

export interface UpdateRuleRequest {
  rule_type?: RuleType;
  title?: string;
  description?: string | null;
  order_position?: number;
}

export interface TagTradeRequest {
  trade_id: number;
  setup_id: string;
  trade_type: TradeTypeForTagging;
}

export interface PlaybookTradesResponse {
  stock_trades: number[];
  option_trades: number[];
}

export interface MissedTrade {
  id: string;
  playbook_id: string;
  symbol: string;
  trade_type: string;
  reason: string;
  potential_entry_price?: number | null;
  opportunity_date: string;
  notes?: string | null;
  created_at: string;
}

export interface CreateMissedTradeRequest {
  playbook_id: string;
  symbol: string;
  trade_type: string;
  reason: string;
  potential_entry_price?: number | null;
  opportunity_date: string;
  notes?: string | null;
}

export interface PlaybookAnalytics {
  playbook_id: string;
  playbook_name: string;
  total_trades: number;
  stock_trades: number;
  option_trades: number;
  win_rate: number;
  net_pnl: number;
  missed_trades: number;
  profit_factor: number;
  expectancy: number;
  average_winner: number;
  average_loser: number;
  total_trades_with_compliance_data: number;
  fully_compliant_trades: number;
  fully_compliant_win_rate: number;
  partially_compliant_trades: number;
  non_compliant_trades: number;
}

export interface ApiResponse<T> {
  success: boolean;
  message: string;
  data?: T;
}

export interface PlaybookListResponse {
  success: boolean;
  message: string;
  data?: Playbook[];
  total?: number;
  page?: number;
  page_size?: number;
}

export interface TagTradeResponse {
  success: boolean;
  message: string;
  data?: PlaybookTradesResponse | unknown;
}

// Hook return types
export interface UsePlaybookReturn {
  playbook: Playbook | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UsePlaybookListReturn {
  playbooks: Playbook[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UsePlaybookRulesReturn {
  rules: PlaybookRule[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseMissedTradesReturn {
  missedTrades: MissedTrade[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UsePlaybookAnalyticsReturn {
  analytics: PlaybookAnalytics | null;
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseAllPlaybooksAnalyticsReturn {
  analytics: PlaybookAnalytics[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

export interface UseTradePlaybooksReturn {
  playbooks: Playbook[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

