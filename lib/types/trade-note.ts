export interface TradeNote {
  id: string;
  name: string;
  content: string;
  tradeType?: 'stock' | 'option';
  stockTradeId?: number;
  optionTradeId?: number;
  aiMetadata?: string; // JSON string
  createdAt: string;
  updatedAt: string;
}

export interface CreateTradeNoteRequest {
  name: string;
  content: string;
}

export interface UpdateTradeNoteRequest {
  name?: string;
  content?: string;
}

export interface CreateTradeNoteForTradeRequest {
  content: string;
}

export interface TradeNoteResponse {
  success: boolean;
  message: string;
  data?: TradeNote;
}

export interface TradeNoteListResponse {
  success: boolean;
  message: string;
  data?: TradeNote[];
  total?: number;
  page?: number;
  pageSize?: number;
}

