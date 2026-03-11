export type OptionTradeStatus = 'open' | 'closed';
export type OptionType = 'Call' | 'Put';

export interface OptionTrade {
  id: number;
  symbol: string;
  optionType: OptionType;
  strikePrice: string; // DECIMAL as string
  expirationDate: string; // ISO
  entryPrice: string; // DECIMAL as string
  exitPrice: string; // DECIMAL as string - required
  premium: string; // DECIMAL as string (renamed from totalPremium)
  entryDate: string; // ISO
  exitDate: string; // ISO - required
  status: OptionTradeStatus;
  initialTarget?: string | null;
  profitTarget?: string | null;
  tradeRatings?: number | null; // 1-5 stars
  createdAt: string;
  updatedAt: string;
  reviewed: boolean;
  mistakes?: string | null;
  brokerageName?: string | null;
  commissions: string; // DECIMAL as string
  // Multi-entry/exit trade linking fields
  tradeGroupId?: string | null;
  parentTradeId?: number | null;
  totalQuantity?: string | null; // Total quantity (contracts) - renamed from quantity
  transactionSequence?: number | null; // Order within trade group (1, 2, 3...)
  isDeleted: boolean; // Soft delete flag
}

export interface CreateOptionRequest {
  symbol: string;
  optionType: OptionType;
  strikePrice: number;
  expirationDate: string; // ISO
  entryPrice: number;
  exitPrice: number; // Required
  premium: number; // Renamed from totalPremium
  commissions?: number; // Defaults to 0.00
  entryDate: string; // ISO
  exitDate: string; // ISO - required
  initialTarget?: number;
  profitTarget?: number;
  tradeRatings?: number; // 1-5 stars
  reviewed?: boolean;
  mistakes?: string;
  brokerageName?: string;
  // Multi-entry/exit trade linking fields
  tradeGroupId?: string;
  parentTradeId?: number;
  totalQuantity?: number; // Total quantity (contracts) - renamed from quantity
  transactionSequence?: number;
}

export interface UpdateOptionRequest {
  symbol?: string;
  optionType?: OptionType;
  strikePrice?: number;
  expirationDate?: string;
  entryPrice?: number;
  exitPrice?: number | null;
  premium?: number; // Renamed from totalPremium
  commissions?: number;
  entryDate?: string;
  exitDate?: string | null;
  status?: OptionTradeStatus;
  initialTarget?: number | null;
  profitTarget?: number | null;
  tradeRatings?: number | null; // 1-5 stars
  reviewed?: boolean;
  mistakes?: string | null;
  brokerageName?: string | null;
  // Multi-entry/exit trade linking fields
  tradeGroupId?: string | null;
  parentTradeId?: number | null;
  totalQuantity?: number | null; // Total quantity (contracts) - renamed from quantity
  transactionSequence?: number | null;
}


