// =====================================================
// Brokerage Types (from backend/src/routes/brokerage.rs)
// =====================================================

/**
 * API Response wrapper
 */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string;
}

/**
 * Request to initiate brokerage connection
 */
export interface ConnectBrokerageRequest {
  brokerage_id: string;
  connection_type?: 'read' | 'trade';
  redirect_uri?: string;
}

/**
 * Response from initiating brokerage connection
 */
export interface ConnectBrokerageResponse {
  redirect_url: string;
  connection_id: string;
}

/**
 * Brokerage connection status
 */
export type ConnectionStatus = 'pending' | 'connected' | 'disconnected' | 'error';

/**
 * Brokerage connection
 */
export interface BrokerageConnection {
  id: string;
  connection_id?: string;
  brokerage_name: string;
  status: ConnectionStatus;
  last_sync_at?: string;
  created_at: string;
  updated_at: string;
}

/**
 * Sync accounts response
 */
export interface SyncAccountsResponse {
  accounts: unknown[];
  holdings: unknown[];
  transactions: unknown[];
}

/**
 * Sync summary
 */
export interface SyncSummary {
  accounts_synced: number;
  holdings_synced: number;
  transactions_synced: number;
  last_sync_at: string;
}

/**
 * Brokerage account
 */
export interface BrokerageAccount {
  id: string;
  connection_id: string;
  snaptrade_account_id: string;
  account_number?: string;
  account_name?: string;
  account_type?: string;
  balance?: number;
  currency?: string;
  institution_name?: string;
  created_at: string;
  updated_at: string;
}

/**
 * Brokerage transaction
 */
export interface BrokerageTransaction {
  id: string;
  account_id: string;
  snaptrade_transaction_id: string;
  symbol?: string;
  transaction_type?: string;
  quantity?: number;
  price?: number;
  amount?: number;
  currency?: string;
  trade_date: string;
  settlement_date?: string;
  fees?: number;
  created_at: string;
  is_transformed?: boolean;
  raw_data?: string;
}

/**
 * Brokerage holding
 */
export interface BrokerageHolding {
  id: string;
  account_id: string;
  symbol: string;
  quantity: number;
  average_cost?: number;
  current_price?: number;
  market_value?: number;
  currency?: string;
  last_updated: string;
}

/**
 * Query parameters for transactions
 */
export interface GetTransactionsQuery {
  account_id?: string;
}

/**
 * Query parameters for holdings
 */
export interface GetHoldingsQuery {
  account_id?: string;
}

/**
 * Connection status response (from SnapTrade)
 */
export interface ConnectionStatusResponse {
  status: string;
  [key: string]: unknown;
}

// =====================================================
// Hook Return Types
// =====================================================

/**
 * Return type for useBrokerageConnections hook
 */
export interface UseBrokerageConnectionsReturn {
  connections: BrokerageConnection[];
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

/**
 * Return type for useBrokerageAccounts hook
 */
export interface UseBrokerageAccountsReturn {
  accounts: BrokerageAccount[];
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

/**
 * Return type for useBrokerageTransactions hook
 */
export interface UseBrokerageTransactionsReturn {
  transactions: BrokerageTransaction[];
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

/**
 * Return type for useBrokerageHoldings hook
 */
export interface UseBrokerageHoldingsReturn {
  holdings: BrokerageHolding[];
  loading: boolean;
  error: Error | null;
  refetch: () => void;
}

/**
 * Unmatched transaction
 */
export interface UnmatchedTransaction {
  id: string;
  user_id: string;
  transaction_id: string;
  snaptrade_transaction_id: string;
  symbol: string;
  trade_type: string;
  units: number;
  price: number;
  fee: number;
  trade_date: string;
  brokerage_name?: string;
  is_option: boolean;
  difficulty_reason?: string;
  confidence_score?: number;
  suggested_matches?: string[];
  status: string;
  resolved_trade_id?: number;
  resolved_at?: string;
  created_at: string;
  updated_at: string;
}

/**
 * Resolve unmatched transaction request
 */
export interface ResolveUnmatchedRequest {
  matched_transaction_id?: string;
  action: 'merge' | 'create_open';
  entry_price?: number;
  entry_date?: string;
}

/**
 * Resolve unmatched transaction response
 */
export interface ResolveUnmatchedResponse {
  trade_id: number;
  message: string;
}

/**
 * Unmatched transaction suggestion
 */
export interface UnmatchedSuggestion {
  id: string;
  symbol: string;
  trade_type: string;
  units: number;
  price: number;
  trade_date: string;
  brokerage_name?: string;
  confidence_score?: number;
  difficulty_reason?: string;
}

/**
 * Account detail response (from SnapTrade)
 */
export interface AccountDetailResponse {
  [key: string]: unknown;
}

/**
 * Account positions response (from SnapTrade)
 */
export interface AccountPositionsResponse {
  positions: unknown[];
}

/**
 * Query parameters for account transactions
 */
export interface GetAccountTransactionsQuery {
  start_date?: string;
  end_date?: string;
  offset?: number;
  limit?: number;
}

/**
 * Request to merge transactions into a stock or option trade
 */
export interface MergeTransactionsRequest {
  transactionIds: string[];
  tradeType: 'stock' | 'option';
  // Stock fields
  symbol: string;
  orderType: string;
  stopLoss?: number;
  takeProfit?: number;
  initialTarget?: number;
  profitTarget?: number;
  tradeRatings?: number;
  reviewed?: boolean;
  mistakes?: string;
  brokerageName?: string;
  // Option-specific fields
  strategyType?: string;
  tradeDirection?: string;
  optionType?: string;
  strikePrice?: number;
  expirationDate?: string;
  impliedVolatility?: number;
}

/**
 * Return type for useBrokerage hook (main hook)
 */
export interface UseBrokerageReturn {
  // Connections
  connections: BrokerageConnection[];
  connectionsLoading: boolean;
  connectionsError: Error | null;
  refetchConnections: () => void;
  
  // Initiate connection
  initiateConnection: (request: ConnectBrokerageRequest) => Promise<ConnectBrokerageResponse>;
  initiating: boolean;
  
  // Connection status
  getConnectionStatus: (connectionId: string) => Promise<ConnectionStatusResponse>;
  statusLoading: boolean;
  
  // Delete connection
  deleteConnection: (connectionId: string) => Promise<void>;
  deleting: boolean;
  
  // Complete connection sync
  completeConnectionSync: (connectionId: string) => Promise<SyncSummary>;
  completingSync: boolean;
  
  // Accounts
  accounts: BrokerageAccount[];
  accountsLoading: boolean;
  accountsError: Error | null;
  refetchAccounts: () => void;
  
  // Get account detail
  getAccountDetail: (accountId: string) => Promise<AccountDetailResponse>;
  accountDetailLoading: boolean;
  
  // Get account positions
  getAccountPositions: (accountId: string) => Promise<AccountPositionsResponse>;
  accountPositionsLoading: boolean;
  
  // Get account option positions
  getAccountOptionPositions: (accountId: string) => Promise<AccountPositionsResponse>;
  accountOptionPositionsLoading: boolean;
  
  // Get account transactions
  getAccountTransactions: (accountId: string, query?: GetAccountTransactionsQuery) => Promise<unknown>;
  accountTransactionsLoading: boolean;
  
  // Sync accounts
  syncAccounts: () => Promise<SyncSummary>;
  syncing: boolean;
  
  // Transactions
  transactions: BrokerageTransaction[];
  transactionsLoading: boolean;
  transactionsError: Error | null;
  refetchTransactions: (query?: GetTransactionsQuery) => void;
  
  // Holdings
  holdings: BrokerageHolding[];
  holdingsLoading: boolean;
  holdingsError: Error | null;
  refetchHoldings: (query?: GetHoldingsQuery) => void;
  
  // Unmatched transactions
  unmatchedTransactions: UnmatchedTransaction[];
  unmatchedTransactionsLoading: boolean;
  unmatchedTransactionsError: Error | null;
  refetchUnmatchedTransactions: () => void;
  
  // Resolve unmatched transaction
  resolveUnmatchedTransaction: (id: string, request: ResolveUnmatchedRequest) => Promise<ResolveUnmatchedResponse>;
  resolvingUnmatched: boolean;
  
  // Ignore unmatched transaction
  ignoreUnmatchedTransaction: (id: string) => Promise<void>;
  ignoringUnmatched: boolean;
  
  // Get unmatched suggestions
  getUnmatchedSuggestions: (id: string) => Promise<UnmatchedSuggestion[]>;
  unmatchedSuggestionsLoading: boolean;
}

